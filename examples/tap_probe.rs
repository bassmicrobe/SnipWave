//! T1b: macOS で自前の Process Tap と集約デバイスを作り、cpal から名前で探して開けるかを検証するプローブ。
//!
//! 使い方:
//!   cargo build --example tap_probe
//!   scripts/macos/probe-app.sh tap build        # TapProbe.app を組み立てて ad-hoc 署名する
//!   scripts/macos/probe-app.sh tap run          # TapProbe.app を起動して probe.log / probe.wav を残す
//!   scripts/macos/probe-app.sh tap rebuild      # ビルドスタンプを更新して組み立て・再署名する
//!   scripts/macos/probe-app.sh tap rebuild-run  # ビルドスタンプを更新して再署名・起動する

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("tap_probe は macOS 専用です。");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    match macos::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("エラー: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::f64::consts::TAU;
    use std::ffi::{c_void, CStr};
    use std::fs;
    use std::io::ErrorKind;
    use std::mem::{self, MaybeUninit};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command};
    use std::ptr::{self, NonNull};
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_core_audio::{
        kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey,
        kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey,
        kAudioAggregateDeviceUIDKey, kAudioDevicePropertyDeviceUID, kAudioHardwareNoError,
        kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyElementMain,
        kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
        kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey, AudioDeviceID,
        AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
        AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap,
        AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress, CATapDescription,
        CATapMuteBehavior,
    };
    use objc2_core_foundation::{
        kCFAllocatorDefault, kCFTypeArrayCallBacks, kCFTypeDictionaryKeyCallBacks,
        kCFTypeDictionaryValueCallBacks, CFArray, CFDictionary, CFMutableDictionary, CFRetained,
        CFString,
    };
    use objc2_foundation::{NSArray, NSNumber, NSString};

    const BUILD_STAMP: &str = match option_env!("SNIPWAVE_PROBE_BUILD_STAMP") {
        Some(stamp) => stamp,
        None => "default",
    };

    const CAPTURE_SECONDS: u64 = 10;
    const BUFFER_CAPACITY_SECONDS: usize = 15;
    const TONE_START_SECOND: u64 = 2;
    const TONE_DURATION_SECONDS: u32 = 3;
    const TONE_FREQ_HZ: f64 = 997.0;
    const TONE_DBFS: f64 = -12.0;
    const TONE_SAMPLE_RATE: u32 = 48_000;
    const DEVICE_LOOKUP_ATTEMPTS: usize = 10;
    const DEVICE_LOOKUP_INTERVAL_MS: u64 = 50;

    const STREAM_ERROR_NONE: u8 = 0;
    const STREAM_ERROR_DEVICE_BUSY: u8 = 1;
    const STREAM_ERROR_DEVICE_CHANGED: u8 = 2;
    const STREAM_ERROR_DEVICE_NOT_AVAILABLE: u8 = 3;
    const STREAM_ERROR_HOST_UNAVAILABLE: u8 = 4;
    const STREAM_ERROR_INVALID_INPUT: u8 = 5;
    const STREAM_ERROR_PERMISSION_DENIED: u8 = 6;
    const STREAM_ERROR_REALTIME_DENIED: u8 = 7;
    const STREAM_ERROR_RESOURCE_EXHAUSTED: u8 = 8;
    const STREAM_ERROR_STREAM_INVALIDATED: u8 = 9;
    const STREAM_ERROR_UNSUPPORTED_CONFIG: u8 = 10;
    const STREAM_ERROR_UNSUPPORTED_OPERATION: u8 = 11;
    const STREAM_ERROR_XRUN: u8 = 12;
    const STREAM_ERROR_BACKEND_ERROR: u8 = 13;
    const STREAM_ERROR_OTHER: u8 = 14;
    const STREAM_ERROR_UNKNOWN: u8 = 255;

    fn encode_error_kind(kind: cpal::ErrorKind) -> u8 {
        match kind {
            cpal::ErrorKind::DeviceBusy => STREAM_ERROR_DEVICE_BUSY,
            cpal::ErrorKind::DeviceChanged => STREAM_ERROR_DEVICE_CHANGED,
            cpal::ErrorKind::DeviceNotAvailable => STREAM_ERROR_DEVICE_NOT_AVAILABLE,
            cpal::ErrorKind::HostUnavailable => STREAM_ERROR_HOST_UNAVAILABLE,
            cpal::ErrorKind::InvalidInput => STREAM_ERROR_INVALID_INPUT,
            cpal::ErrorKind::PermissionDenied => STREAM_ERROR_PERMISSION_DENIED,
            cpal::ErrorKind::RealtimeDenied => STREAM_ERROR_REALTIME_DENIED,
            cpal::ErrorKind::ResourceExhausted => STREAM_ERROR_RESOURCE_EXHAUSTED,
            cpal::ErrorKind::StreamInvalidated => STREAM_ERROR_STREAM_INVALIDATED,
            cpal::ErrorKind::UnsupportedConfig => STREAM_ERROR_UNSUPPORTED_CONFIG,
            cpal::ErrorKind::UnsupportedOperation => STREAM_ERROR_UNSUPPORTED_OPERATION,
            cpal::ErrorKind::Xrun => STREAM_ERROR_XRUN,
            cpal::ErrorKind::BackendError => STREAM_ERROR_BACKEND_ERROR,
            cpal::ErrorKind::Other => STREAM_ERROR_OTHER,
            _ => STREAM_ERROR_UNKNOWN,
        }
    }

    fn decode_error_kind(code: u8) -> &'static str {
        match code {
            STREAM_ERROR_NONE => "None",
            STREAM_ERROR_DEVICE_BUSY => "DeviceBusy",
            STREAM_ERROR_DEVICE_CHANGED => "DeviceChanged",
            STREAM_ERROR_DEVICE_NOT_AVAILABLE => "DeviceNotAvailable",
            STREAM_ERROR_HOST_UNAVAILABLE => "HostUnavailable",
            STREAM_ERROR_INVALID_INPUT => "InvalidInput",
            STREAM_ERROR_PERMISSION_DENIED => "PermissionDenied",
            STREAM_ERROR_REALTIME_DENIED => "RealtimeDenied",
            STREAM_ERROR_RESOURCE_EXHAUSTED => "ResourceExhausted",
            STREAM_ERROR_STREAM_INVALIDATED => "StreamInvalidated",
            STREAM_ERROR_UNSUPPORTED_CONFIG => "UnsupportedConfig",
            STREAM_ERROR_UNSUPPORTED_OPERATION => "UnsupportedOperation",
            STREAM_ERROR_XRUN => "Xrun",
            STREAM_ERROR_BACKEND_ERROR => "BackendError",
            STREAM_ERROR_OTHER => "Other",
            _ => "Unknown",
        }
    }

    struct Args {
        output: Option<PathBuf>,
        info_only: bool,
    }

    struct CaptureBuffer {
        samples: Box<[AtomicU32]>,
        max_frames: usize,
        written_frames: AtomicUsize,
        dropped_frames: AtomicU64,
        stream_error: AtomicBool,
        stream_error_kind: AtomicU8,
        stream_error_count: AtomicU64,
    }

    impl CaptureBuffer {
        fn new(max_frames: usize) -> Self {
            let zero_bits = 0.0_f32.to_bits();
            let mut vec = Vec::with_capacity(max_frames.saturating_mul(2));
            for _ in 0..max_frames.saturating_mul(2) {
                vec.push(AtomicU32::new(zero_bits));
            }
            Self {
                samples: vec.into_boxed_slice(),
                max_frames,
                written_frames: AtomicUsize::new(0),
                dropped_frames: AtomicU64::new(0),
                stream_error: AtomicBool::new(false),
                stream_error_kind: AtomicU8::new(STREAM_ERROR_NONE),
                stream_error_count: AtomicU64::new(0),
            }
        }

        fn read_frame(&self, frame_index: usize) -> (f32, f32) {
            let base = frame_index * 2;
            let left = f32::from_bits(self.samples[base].load(Ordering::Relaxed));
            let right = f32::from_bits(self.samples[base + 1].load(Ordering::Relaxed));
            (left, right)
        }
    }

    struct TempFileGuard(PathBuf);

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            if let Err(err) = fs::remove_file(&self.0) {
                if err.kind() != ErrorKind::NotFound {
                    eprintln!(
                        "警告: 一時ファイル {} の削除に失敗しました: {err}",
                        self.0.display()
                    );
                }
            }
        }
    }

    struct TapAggregate {
        tap_id: AudioObjectID,
        aggregate_device_id: AudioObjectID,
        device_name: String,
        is_private: bool,
    }

    impl TapAggregate {
        fn create(
            target_device_uid: &str,
            is_private: bool,
            attempt_tag: &str,
        ) -> Result<Self, String> {
            let pid = std::process::id();
            let device_name = format!("SnipWave Loopback {pid}");
            let agg_uid = format!("com.snipwave.LoopbackAggregate.{pid}.{attempt_tag}");
            let tap_name = format!("SnipWave Loopback Tap {pid}.{attempt_tag}");

            let processes: Retained<NSArray<NSNumber>> = NSArray::new();
            let device_uid_ns = NSString::from_str(target_device_uid);

            // SAFETY: `CATapDescription::alloc()` で確保した未初期化インスタンスに対し、
            // 有効な `NSArray<NSNumber>`（空配列）・`NSString`（対象デバイス UID）・
            // ストリーム番号 0 を渡して `initWithProcesses:andDeviceUID:withStream:` を 1 回だけ呼ぶ。
            let tap_desc = unsafe {
                CATapDescription::initWithProcesses_andDeviceUID_withStream(
                    CATapDescription::alloc(),
                    &processes,
                    &device_uid_ns,
                    0,
                )
            };
            // SAFETY: `tap_desc` は初期化済みの有効な `CATapDescription` であり、
            // 各セッタに有効な列挙値・`NSString`・真偽値を渡している。
            unsafe {
                tap_desc.setMuteBehavior(CATapMuteBehavior::Unmuted);
                tap_desc.setName(&NSString::from_str(&tap_name));
                tap_desc.setPrivate(is_private);
                tap_desc.setExclusive(true);
            }

            let mut tap_id_uninit: MaybeUninit<AudioObjectID> = MaybeUninit::uninit();
            // SAFETY: `tap_desc` は初期化済みの有効な `CATapDescription`、
            // `tap_id_uninit.as_mut_ptr()` は書き込み可能な `AudioObjectID` への有効なポインタであり、
            // `status == kAudioHardwareNoError` のときに限って CoreAudio が書き込んだ値を `assume_init()` する。
            let tap_id = unsafe {
                let status = AudioHardwareCreateProcessTap(
                    Some(tap_desc.as_ref()),
                    tap_id_uninit.as_mut_ptr(),
                );
                if status != kAudioHardwareNoError {
                    return Err(format!(
                        "AudioHardwareCreateProcessTap が失敗しました (OSStatus={status})"
                    ));
                }
                tap_id_uninit.assume_init()
            };

            // SAFETY: `tap_desc` は初期化済みの有効な `CATapDescription` であり、
            // `UUID` とその `UUIDString` を読み取るだけである。
            let tap_uid = unsafe { tap_desc.UUID().UUIDString() };
            let properties =
                match build_aggregate_properties(&tap_uid, &agg_uid, &device_name, is_private) {
                    Ok(props) => props,
                    Err(err) => {
                        destroy_tap_or_log(tap_id);
                        return Err(err);
                    }
                };

            let mut aggregate_device_id: AudioObjectID = 0;
            // SAFETY: `properties` は有効な `CFDictionary`、`aggregate_device_id` は
            // 書き込み可能な `AudioObjectID` への有効な非 NULL ポインタである。
            let status = unsafe {
                AudioHardwareCreateAggregateDevice(
                    properties.as_ref(),
                    NonNull::from(&mut aggregate_device_id),
                )
            };
            if status != kAudioHardwareNoError {
                destroy_tap_or_log(tap_id);
                return Err(format!(
                    "AudioHardwareCreateAggregateDevice が失敗しました (OSStatus={status})"
                ));
            }

            Ok(Self {
                tap_id,
                aggregate_device_id,
                device_name,
                is_private,
            })
        }

        fn device_name(&self) -> &str {
            &self.device_name
        }

        fn is_private(&self) -> bool {
            self.is_private
        }
    }

    impl Drop for TapAggregate {
        fn drop(&mut self) {
            // SAFETY: `self.aggregate_device_id` は `AudioHardwareCreateAggregateDevice` が
            // 成功時に返した有効な ID であり、`Drop` は 1 回しか呼ばれないため二重破棄にならない。
            let agg_status =
                unsafe { AudioHardwareDestroyAggregateDevice(self.aggregate_device_id) };
            if agg_status != kAudioHardwareNoError {
                eprintln!(
                    "警告: 集約デバイス '{}' (id={}) の破棄に失敗しました (OSStatus={agg_status})",
                    self.device_name, self.aggregate_device_id
                );
            }
            destroy_tap_or_log(self.tap_id);
        }
    }

    fn destroy_tap_or_log(tap_id: AudioObjectID) {
        // SAFETY: `tap_id` は `AudioHardwareCreateProcessTap` が成功時に返した有効な ID であり、
        // 作成失敗時のロールバックまたは `Drop` のいずれかで 1 回だけ破棄する。
        let tap_status = unsafe { AudioHardwareDestroyProcessTap(tap_id) };
        if tap_status != kAudioHardwareNoError {
            eprintln!(
                "警告: Process Tap (id={tap_id}) の破棄に失敗しました (OSStatus={tap_status})"
            );
        }
    }

    fn to_cfstring(cstr: &'static CStr) -> Result<CFRetained<CFString>, String> {
        // SAFETY: `cstr` は `'static` かつ NUL 終端の有効な UTF-8 C 文字列であり、
        // `kCFAllocatorDefault` と UTF-8 エンコーディングを指定して `CFString::with_c_string` を呼ぶ。
        unsafe {
            CFString::with_c_string(
                kCFAllocatorDefault,
                cstr.as_ptr(),
                0x0800_0100, // kCFStringEncodingUTF8
            )
        }
        .ok_or_else(|| format!("CFString の作成に失敗しました: {cstr:?}"))
    }

    fn build_aggregate_properties(
        tap_uid: &NSString,
        agg_uid: &str,
        agg_name: &str,
        is_private: bool,
    ) -> Result<CFRetained<CFDictionary>, String> {
        // SAFETY: `kCFAllocatorDefault` と CoreFoundation 標準の
        // `kCFTypeDictionaryKeyCallBacks` / `kCFTypeDictionaryValueCallBacks` への
        // 有効な静的参照を渡して可変辞書を作成する。
        let tap_inner = unsafe {
            CFMutableDictionary::new(
                kCFAllocatorDefault,
                2,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
        }
        .ok_or_else(|| "サブタップ辞書の作成に失敗しました".to_string())?;

        let key_sub_tap_uid = to_cfstring(kAudioSubTapUIDKey)?;
        let key_drift = to_cfstring(kAudioSubTapDriftCompensationKey)?;
        let val_yes_drift = NSNumber::initWithBool(NSNumber::alloc(), true);

        // SAFETY: `tap_inner` は `kCFTypeDictionary*CallBacks` で作成した可変辞書であり、
        // キー（`CFString`）と値（Toll-Free Bridging 対象の `NSString` / `NSNumber`）は
        // いずれも呼び出し中有効な CFType ポインタで、辞書側が `CFRetain` する。
        unsafe {
            CFMutableDictionary::set_value(
                Some(tap_inner.as_ref()),
                (&*key_sub_tap_uid as *const CFString).cast(),
                (tap_uid as *const NSString).cast(),
            );
            CFMutableDictionary::set_value(
                Some(tap_inner.as_ref()),
                (&*key_drift as *const CFString).cast(),
                (&*val_yes_drift as *const NSNumber).cast(),
            );
        }

        let tap_inner_ptr: *const c_void = (&*tap_inner as *const CFMutableDictionary).cast();
        let taps_list = [tap_inner_ptr];
        // SAFETY: `taps_list` は有効な CFType（`CFMutableDictionary`）のポインタ 1 要素の配列で、
        // 要素数 `taps_list.len()` と一致し、`kCFTypeArrayCallBacks` により `CFArray::new` 内で
        // 要素が `CFRetain` される（ポインタ配列自体は読み取りのみ）。
        let taps = unsafe {
            CFArray::new(
                kCFAllocatorDefault,
                taps_list.as_ptr().cast_mut(),
                taps_list.len() as _,
                &kCFTypeArrayCallBacks,
            )
        }
        .ok_or_else(|| "サブタップ配列の作成に失敗しました".to_string())?;

        // SAFETY: `kCFAllocatorDefault` と CoreFoundation 標準の
        // `kCFTypeDictionaryKeyCallBacks` / `kCFTypeDictionaryValueCallBacks` への
        // 有効な静的参照を渡して可変辞書を作成する。
        let dict = unsafe {
            CFMutableDictionary::new(
                kCFAllocatorDefault,
                5,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
        }
        .ok_or_else(|| "集約デバイス辞書の作成に失敗しました".to_string())?;

        let key_name = to_cfstring(kAudioAggregateDeviceNameKey)?;
        let key_uid = to_cfstring(kAudioAggregateDeviceUIDKey)?;
        let key_tap_list = to_cfstring(kAudioAggregateDeviceTapListKey)?;
        let key_auto_start = to_cfstring(kAudioAggregateDeviceTapAutoStartKey)?;
        let key_is_private = to_cfstring(kAudioAggregateDeviceIsPrivateKey)?;

        let val_name = CFString::from_str(agg_name);
        let val_uid = CFString::from_str(agg_uid);
        let val_auto_start = NSNumber::initWithBool(NSNumber::alloc(), true);
        let val_is_private = NSNumber::initWithBool(NSNumber::alloc(), is_private);

        // SAFETY: `dict` は `kCFTypeDictionary*CallBacks` で作成した可変辞書であり、
        // 登録するキー（`CFString`）と値（`CFString` / `CFArray` / `NSNumber`）は
        // いずれも呼び出し中有効な CFType ポインタで辞書側が `CFRetain` する。
        // また `CFMutableDictionary` は `CFDictionary` のサブタイプのため `cast_unchecked` は安全。
        unsafe {
            CFMutableDictionary::set_value(
                Some(dict.as_ref()),
                (&*key_name as *const CFString).cast(),
                (&*val_name as *const CFString).cast(),
            );
            CFMutableDictionary::set_value(
                Some(dict.as_ref()),
                (&*key_uid as *const CFString).cast(),
                (&*val_uid as *const CFString).cast(),
            );
            CFMutableDictionary::set_value(
                Some(dict.as_ref()),
                (&*key_tap_list as *const CFString).cast(),
                (&*taps as *const CFArray).cast(),
            );
            CFMutableDictionary::set_value(
                Some(dict.as_ref()),
                (&*key_auto_start as *const CFString).cast(),
                (&*val_auto_start as *const NSNumber).cast(),
            );
            CFMutableDictionary::set_value(
                Some(dict.as_ref()),
                (&*key_is_private as *const CFString).cast(),
                (&*val_is_private as *const NSNumber).cast(),
            );

            Ok(CFRetained::cast_unchecked::<CFDictionary>(dict))
        }
    }

    fn default_output_device_id() -> Result<AudioDeviceID, String> {
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        let mut device_id: AudioDeviceID = 0;
        let mut data_size = mem::size_of::<AudioDeviceID>() as u32;
        // SAFETY: `address` は有効なプロパティアドレス、qualifier は 0 バイト（NULL）、
        // `device_id` は `data_size`（`size_of::<AudioDeviceID>()`）バイトの書き込み可能な領域である。
        let status = unsafe {
            AudioObjectGetPropertyData(
                kAudioObjectSystemObject as AudioObjectID,
                NonNull::from(&address),
                0,
                ptr::null(),
                NonNull::from(&mut data_size),
                NonNull::from(&mut device_id).cast(),
            )
        };
        if status != kAudioHardwareNoError || device_id == 0 {
            return Err(format!(
                "CoreAudio 既定出力デバイス ID の取得に失敗しました (OSStatus={status}, id={device_id})"
            ));
        }
        Ok(device_id)
    }

    fn device_uid(device_id: AudioDeviceID) -> Result<String, String> {
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioDevicePropertyDeviceUID,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        let mut cf_uid: *mut CFString = ptr::null_mut();
        let mut data_size = mem::size_of::<*mut CFString>() as u32;
        // SAFETY: `device_id` は取得済みの出力デバイス ID、`address` は有効なプロパティアドレス、
        // qualifier は 0 バイト（NULL）、`cf_uid` は `data_size`（`size_of::<*mut CFString>()`）
        // バイトの書き込み可能な領域である。
        let status = unsafe {
            AudioObjectGetPropertyData(
                device_id,
                NonNull::from(&address),
                0,
                ptr::null(),
                NonNull::from(&mut data_size),
                NonNull::from(&mut cf_uid).cast(),
            )
        };
        if status != kAudioHardwareNoError {
            return Err(format!(
                "CoreAudio デバイス UID の取得に失敗しました (OSStatus={status})"
            ));
        }
        let non_null = NonNull::new(cf_uid)
            .ok_or_else(|| "CoreAudio デバイス UID が NULL でした".to_string())?;
        // SAFETY: `AudioObjectGetPropertyData`（`kAudioDevicePropertyDeviceUID`）は成功時に
        // Create Rule に従い +1 所有権を持つ有効な `CFString` を返し、`non_null` は非 NULL 確認済み。
        let uid = unsafe { CFRetained::from_raw(non_null) };
        Ok(uid.to_string())
    }

    fn find_device_by_name_with_retry(
        host: &cpal::Host,
        target_name: &str,
    ) -> Result<Option<cpal::Device>, String> {
        for attempt in 0..DEVICE_LOOKUP_ATTEMPTS {
            let devices = host
                .devices()
                .map_err(|e| format!("cpal デバイス一覧の取得に失敗しました: {e}"))?;
            for dev in devices {
                if dev.to_string() == target_name {
                    return Ok(Some(dev));
                }
            }
            if attempt + 1 < DEVICE_LOOKUP_ATTEMPTS {
                std::thread::sleep(Duration::from_millis(DEVICE_LOOKUP_INTERVAL_MS));
            }
        }
        Ok(None)
    }

    fn create_and_find_tap_aggregate(
        host: &cpal::Host,
        target_device_uid: &str,
    ) -> Result<(TapAggregate, cpal::Device), String> {
        println!(
            "Step 1: 非公開の集約デバイス (kAudioAggregateDeviceIsPrivateKey = YES) を作成します"
        );
        let private_agg = TapAggregate::create(target_device_uid, true, "private")?;
        println!(
            "created_aggregate (private): name='{}', agg_id={}, tap_id={}",
            private_agg.device_name(),
            private_agg.aggregate_device_id,
            private_agg.tap_id
        );

        if let Some(dev) = find_device_by_name_with_retry(host, private_agg.device_name())? {
            println!("aggregate_visibility: private (kAudioAggregateDeviceIsPrivateKey = YES) で cpal::Host::devices() に出現しました");
            return Ok((private_agg, dev));
        }

        println!(
            "aggregate_visibility: private では cpal::Host::devices() に出現しませんでした。非公開の集約デバイスと Tap を破棄し、公開で作り直します。"
        );
        drop(private_agg);

        println!(
            "Step 2: 公開の集約デバイス (kAudioAggregateDeviceIsPrivateKey = NO) を作成します"
        );
        let public_agg = TapAggregate::create(target_device_uid, false, "public")?;
        println!(
            "created_aggregate (public): name='{}', agg_id={}, tap_id={}",
            public_agg.device_name(),
            public_agg.aggregate_device_id,
            public_agg.tap_id
        );

        if let Some(dev) = find_device_by_name_with_retry(host, public_agg.device_name())? {
            println!("aggregate_visibility: public (kAudioAggregateDeviceIsPrivateKey = NO) で cpal::Host::devices() に出現しました");
            return Ok((public_agg, dev));
        }

        let missing_name = public_agg.device_name().to_string();
        drop(public_agg);
        Err(format!(
            "非公開・公開のどちらでも集約デバイス '{missing_name}' が cpal::Host::devices() に見つかりませんでした"
        ))
    }

    struct WindowMetrics {
        frames: usize,
        peak_l: f32,
        peak_r: f32,
        peak_max: f32,
        rms: f64,
        amp_997: f64,
        ratio_997: f64,
    }

    pub fn run() -> Result<(), String> {
        let args = parse_args()?;
        let host = cpal::default_host();
        let default_output = host
            .default_output_device()
            .ok_or_else(|| "既定の出力デバイスが見つかりません".to_string())?;

        let default_output_name = default_output.to_string();
        let default_output_cpal_id = default_output
            .id()
            .map(|id| id.to_string())
            .unwrap_or_else(|e| format!("<取得失敗: {e}>"));
        let default_supports_input = default_output.supports_input();
        let default_supports_output = default_output.supports_output();

        let coreaudio_out_id = default_output_device_id()?;
        let coreaudio_out_uid = device_uid(coreaudio_out_id)?;

        println!("=== tap_probe (T1b) ===");
        println!("build_stamp: {BUILD_STAMP}");
        println!("default_output_device: {default_output_name}");
        println!("default_output_cpal_id: {default_output_cpal_id}");
        println!("default_output_coreaudio_id: {coreaudio_out_id}");
        println!("default_output_uid: {coreaudio_out_uid}");
        println!("default_output_supports_input: {default_supports_input}");
        println!("default_output_supports_output: {default_supports_output}");

        let default_output_cfg = default_output
            .default_output_config()
            .map_err(|e| format!("既定出力デバイスの default_output_config 取得に失敗: {e}"))?;
        println!(
            "default_output_config (before): rate={} Hz, channels={}, sample_format={:?}, buffer_size={:?}",
            default_output_cfg.sample_rate(),
            default_output_cfg.channels(),
            default_output_cfg.sample_format(),
            default_output_cfg.buffer_size()
        );

        if args.info_only {
            println!("--info-only のため、Tap とストリームを作らずに終了します。");
            return Ok(());
        }

        let output_wav_path = resolve_output_path(args.output)?;
        println!("output_wav_path: {}", output_wav_path.display());

        let (tap_aggregate, agg_device) = create_and_find_tap_aggregate(&host, &coreaudio_out_uid)?;
        let created_name = tap_aggregate.device_name().to_string();
        let opened_visibility = if tap_aggregate.is_private() {
            "private"
        } else {
            "public"
        };

        let agg_supports_input = agg_device.supports_input();
        let agg_id = agg_device
            .id()
            .map(|id| id.to_string())
            .unwrap_or_else(|e| format!("<取得失敗: {e}>"));
        println!("opened_aggregate_name: {created_name}");
        println!("opened_aggregate_visibility: {opened_visibility}");
        println!("opened_aggregate_cpal_id: {agg_id}");
        println!("opened_aggregate_supports_input: {agg_supports_input}");

        if !agg_supports_input {
            return Err(format!(
                "集約デバイス '{created_name}' が supports_input() == false のため入力として開けません"
            ));
        }

        let supported_config = agg_device
            .default_input_config()
            .map_err(|e| format!("集約デバイスの default_input_config 取得に失敗: {e}"))?;
        let sample_rate = supported_config.sample_rate();
        let input_channels = usize::from(supported_config.channels());
        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();

        println!(
            "aggregate_default_input_config: rate={} Hz, channels={}, sample_format={:?}, buffer_size={:?}",
            supported_config.sample_rate(),
            supported_config.channels(),
            supported_config.sample_format(),
            supported_config.buffer_size()
        );

        let tone_path =
            std::env::temp_dir().join(format!("snipwave_t1b_tone_{}.wav", std::process::id()));
        write_test_tone_wav(&tone_path)?;
        let _tone_guard = TempFileGuard(tone_path.clone());
        println!(
            "test_tone_wav: {} ({} Hz, {} dBFS, {} s)",
            tone_path.display(),
            TONE_FREQ_HZ,
            TONE_DBFS,
            TONE_DURATION_SECONDS
        );

        let capacity_frames = (sample_rate as usize).saturating_mul(BUFFER_CAPACITY_SECONDS);
        let capture = Arc::new(CaptureBuffer::new(capacity_frames));

        let stream = build_capture_stream(
            &agg_device,
            stream_config,
            sample_format,
            input_channels,
            Arc::clone(&capture),
        )?;
        stream
            .play()
            .map_err(|e| format!("集約デバイスストリームの開始に失敗しました: {e}"))?;

        let start = Instant::now();
        let mut frame_marks = [0usize; (CAPTURE_SECONDS as usize) + 1];
        frame_marks[0] = capture.written_frames.load(Ordering::Acquire);
        let mut prev_frames = frame_marks[0];
        let mut afplay_child: Option<Child> = None;

        for sec in 1..=CAPTURE_SECONDS {
            let target = start + Duration::from_secs(sec);
            let now = Instant::now();
            if let Some(wait) = target.checked_duration_since(now) {
                std::thread::sleep(wait);
            }

            let current_frames = capture.written_frames.load(Ordering::Acquire);
            let delta = current_frames.saturating_sub(prev_frames);
            let (peak_l, peak_r) = peak_range(&capture, prev_frames, current_frames);
            println!(
                "[{sec:>2}s] frames={delta:>6} total_frames={current_frames:>7} peak_L={peak_l:.6} peak_R={peak_r:.6}"
            );

            frame_marks[sec as usize] = current_frames;
            prev_frames = current_frames;

            if sec == TONE_START_SECOND {
                println!(
                    "試験音を再生します: /usr/bin/afplay {}",
                    tone_path.display()
                );
                let child = Command::new("/usr/bin/afplay")
                    .arg(&tone_path)
                    .spawn()
                    .map_err(|e| format!("afplay の起動に失敗しました: {e}"))?;
                afplay_child = Some(child);
            }
        }

        // ストリームを先に止め、その後で集約デバイスと Tap を破棄する
        drop(stream);
        drop(tap_aggregate);

        // 破棄後、少し待ってから cpal::Host::devices() に残っていないことを確認
        std::thread::sleep(Duration::from_millis(100));
        let devices_after = host
            .devices()
            .map_err(|e| format!("破棄後のデバイス一覧取得に失敗しました: {e}"))?;
        let still_present = devices_after
            .into_iter()
            .any(|d| d.to_string() == created_name);
        println!("cleanup_check: aggregate_device_still_present={still_present}");

        if let Some(mut child) = afplay_child {
            let status = child
                .wait()
                .map_err(|e| format!("afplay の終了待ちに失敗しました: {e}"))?;
            println!("afplay_exit_status: {status}");
            if !status.success() {
                return Err(format!("afplay が異常終了しました: {status}"));
            }
        }

        let total_frames = capture.written_frames.load(Ordering::Acquire);
        let dropped_frames = capture.dropped_frames.load(Ordering::Relaxed);
        let stream_error = capture.stream_error.load(Ordering::Acquire);
        let stream_error_kind =
            decode_error_kind(capture.stream_error_kind.load(Ordering::Acquire));
        let stream_error_count = capture.stream_error_count.load(Ordering::Relaxed);
        println!("total_frames: {total_frames}");
        println!("dropped_frames: {dropped_frames}");
        println!("stream_error: {stream_error}");
        println!("stream_error_kind: {stream_error_kind}");
        println!("stream_error_count: {stream_error_count}");

        match default_output.default_output_config() {
            Ok(after_cfg) => {
                println!(
                    "default_output_config (after): rate={} Hz, channels={}, sample_format={:?}, buffer_size={:?}",
                    after_cfg.sample_rate(),
                    after_cfg.channels(),
                    after_cfg.sample_format(),
                    after_cfg.buffer_size()
                );
            }
            Err(e) => {
                println!("default_output_config (after): 取得失敗 ({e})");
            }
        }

        write_probe_wav(&output_wav_path, &capture, total_frames, sample_rate)?;
        println!("wrote_wav: {}", output_wav_path.display());

        let margin_02 = (sample_rate as usize) / 5;
        let margin_05 = (sample_rate as usize) / 2;

        let pre_raw_start = frame_marks[0];
        let pre_raw_end = frame_marks[TONE_START_SECOND as usize].min(total_frames);
        let (pre_start, pre_end) = shrink_window(pre_raw_start, pre_raw_end, margin_02, margin_02);

        let tone_end_sec = (TONE_START_SECOND as usize) + (TONE_DURATION_SECONDS as usize);
        let tone_raw_start = frame_marks[TONE_START_SECOND as usize].min(total_frames);
        let tone_raw_end = frame_marks[tone_end_sec].min(total_frames);
        let (tone_start, tone_end) =
            shrink_window(tone_raw_start, tone_raw_end, margin_05, margin_02);

        let pre_metrics = analyze_window(&capture, pre_start, pre_end, sample_rate, TONE_FREQ_HZ);
        let tone_metrics =
            analyze_window(&capture, tone_start, tone_end, sample_rate, TONE_FREQ_HZ);

        println!(
            "pre_tone_window ({pre_start}..{pre_end}): frames={} peak_L={:.6} peak_R={:.6} peak_max={:.6} rms={:.6} amp_997={:.6} ratio_997={:.6}",
            pre_metrics.frames,
            pre_metrics.peak_l,
            pre_metrics.peak_r,
            pre_metrics.peak_max,
            pre_metrics.rms,
            pre_metrics.amp_997,
            pre_metrics.ratio_997
        );
        println!(
            "tone_window ({tone_start}..{tone_end}): frames={} peak_L={:.6} peak_R={:.6} peak_max={:.6} rms={:.6} amp_997={:.6} ratio_997={:.6}",
            tone_metrics.frames,
            tone_metrics.peak_l,
            tone_metrics.peak_r,
            tone_metrics.peak_max,
            tone_metrics.rms,
            tone_metrics.amp_997,
            tone_metrics.ratio_997
        );
        println!(
            "check: opened_visibility={opened_visibility} tone_peak_gt_zero={} tone_ratio_ge_0_7={} cleanup_ok={}",
            tone_metrics.peak_max > 0.0,
            tone_metrics.ratio_997 >= 0.7,
            !still_present
        );

        Ok(())
    }

    fn parse_args() -> Result<Args, String> {
        let mut output = None;
        let mut info_only = false;
        let mut iter = std::env::args().skip(1);

        while let Some(arg) = iter.next() {
            if arg == "--info-only" {
                info_only = true;
            } else if arg == "--output" {
                let value = iter
                    .next()
                    .ok_or_else(|| "--output の後にパスを指定してください".to_string())?;
                output = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--output=") {
                output = Some(PathBuf::from(value));
            } else if arg.starts_with("-psn_") {
                // macOS LaunchServices が渡すプロセスシリアル番号は無視する
            } else {
                return Err(format!("不明な引数です: {arg}"));
            }
        }

        Ok(Args { output, info_only })
    }

    fn resolve_output_path(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
        let path = match explicit {
            Some(p) => p,
            None => default_output_path()?,
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "出力先フォルダ {} の作成に失敗しました: {e}",
                    parent.display()
                )
            })?;
        }
        Ok(path)
    }

    fn default_output_path() -> Result<PathBuf, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("実行ファイルパスの取得に失敗しました: {e}"))?;

        for ancestor in exe.ancestors() {
            if ancestor
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "app")
            {
                if let Some(parent) = ancestor.parent() {
                    return Ok(parent.join("tap").join("probe.wav"));
                }
            }
        }

        // target/<profile>/examples/tap_probe -> target/probe/tap/probe.wav
        if let Some(target_dir) = exe.parent().and_then(Path::parent).and_then(Path::parent) {
            if target_dir.file_name().and_then(|n| n.to_str()) == Some("target") {
                return Ok(target_dir.join("probe").join("tap").join("probe.wav"));
            }
        }

        Ok(std::env::temp_dir().join("snipwave_tap_probe.wav"))
    }

    fn write_test_tone_wav(path: &Path) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: TONE_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("試験音 WAV {} の作成に失敗しました: {e}", path.display()))?;

        let total_frames = (TONE_SAMPLE_RATE as usize) * (TONE_DURATION_SECONDS as usize);
        let fade_frames = (TONE_SAMPLE_RATE as usize) / 200; // 5 ms
        let amplitude = 10.0_f64.powf(TONE_DBFS / 20.0);

        for i in 0..total_frames {
            let phase = TAU * TONE_FREQ_HZ * (i as f64) / f64::from(TONE_SAMPLE_RATE);
            let env = if i < fade_frames {
                i as f64 / fade_frames as f64
            } else if i + fade_frames >= total_frames {
                (total_frames - 1 - i) as f64 / fade_frames as f64
            } else {
                1.0
            };
            let sample_f64 = phase.sin() * amplitude * env;
            let sample_i16 = (sample_f64 * f64::from(i16::MAX))
                .round()
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX))
                as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("試験音サンプルの書き込みに失敗しました: {e}"))?;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("試験音サンプルの書き込みに失敗しました: {e}"))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("試験音 WAV の確定に失敗しました: {e}"))?;
        Ok(())
    }

    fn build_capture_stream(
        device: &cpal::Device,
        config: StreamConfig,
        sample_format: SampleFormat,
        input_channels: usize,
        capture: Arc<CaptureBuffer>,
    ) -> Result<Stream, String> {
        match sample_format {
            SampleFormat::F32 => build_typed_stream::<f32>(device, config, input_channels, capture),
            SampleFormat::I16 => build_typed_stream::<i16>(device, config, input_channels, capture),
            SampleFormat::I32 => build_typed_stream::<i32>(device, config, input_channels, capture),
            SampleFormat::U16 => build_typed_stream::<u16>(device, config, input_channels, capture),
            other => Err(format!("未対応のサンプル形式です: {other:?}")),
        }
    }

    fn build_typed_stream<T>(
        device: &cpal::Device,
        config: StreamConfig,
        input_channels: usize,
        capture: Arc<CaptureBuffer>,
    ) -> Result<Stream, String>
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        let data_capture = Arc::clone(&capture);
        let err_capture = capture;

        device
            .build_input_stream(
                config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    if input_channels == 0 {
                        return;
                    }
                    let mut frame_pos = data_capture.written_frames.load(Ordering::Relaxed);
                    let max_frames = data_capture.max_frames;
                    let mut dropped = 0u64;

                    for frame in data.chunks_exact(input_channels) {
                        let left = f32::from_sample(frame[0]);
                        let right = if input_channels >= 2 {
                            f32::from_sample(frame[1])
                        } else {
                            left
                        };
                        if frame_pos < max_frames {
                            let base = frame_pos * 2;
                            data_capture.samples[base].store(left.to_bits(), Ordering::Relaxed);
                            data_capture.samples[base + 1]
                                .store(right.to_bits(), Ordering::Relaxed);
                            frame_pos += 1;
                        } else {
                            dropped += 1;
                        }
                    }

                    data_capture
                        .written_frames
                        .store(frame_pos, Ordering::Release);
                    if dropped > 0 {
                        data_capture
                            .dropped_frames
                            .fetch_add(dropped, Ordering::Relaxed);
                    }
                },
                move |err| {
                    let kind_code = encode_error_kind(err.kind());
                    // 最初のエラー種別のみ 1 回だけ記録する（既に記録済みで Err になる場合は上書きしない）
                    if err_capture
                        .stream_error_kind
                        .compare_exchange(
                            STREAM_ERROR_NONE,
                            kind_code,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        // 初回のエラー種別を記録した
                    }
                    err_capture
                        .stream_error_count
                        .fetch_add(1, Ordering::Relaxed);
                    err_capture.stream_error.store(true, Ordering::Release);
                },
                None,
            )
            .map_err(|e| format!("入力ストリームの作成に失敗しました: {e}"))
    }

    fn peak_range(capture: &CaptureBuffer, start: usize, end: usize) -> (f32, f32) {
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;
        for frame in start..end {
            let (l, r) = capture.read_frame(frame);
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
        }
        (peak_l, peak_r)
    }

    fn shrink_window(
        start: usize,
        end: usize,
        lead_margin: usize,
        tail_margin: usize,
    ) -> (usize, usize) {
        let shrunk_start = start.saturating_add(lead_margin);
        let shrunk_end = end.saturating_sub(tail_margin);
        if shrunk_start < shrunk_end {
            (shrunk_start, shrunk_end)
        } else {
            (start, end)
        }
    }

    fn analyze_window(
        capture: &CaptureBuffer,
        start: usize,
        end: usize,
        sample_rate: u32,
        target_hz: f64,
    ) -> WindowMetrics {
        let frames = end.saturating_sub(start);
        if frames == 0 || sample_rate == 0 {
            return WindowMetrics {
                frames: 0,
                peak_l: 0.0,
                peak_r: 0.0,
                peak_max: 0.0,
                rms: 0.0,
                amp_997: 0.0,
                ratio_997: 0.0,
            };
        }

        let omega = TAU * target_hz / f64::from(sample_rate);
        let coeff = 2.0 * omega.cos();
        let mut s1 = 0.0_f64;
        let mut s2 = 0.0_f64;
        let mut sum_sq = 0.0_f64;
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;

        for frame in start..end {
            let (l, r) = capture.read_frame(frame);
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
            let mono = (f64::from(l) + f64::from(r)) * 0.5;
            sum_sq += mono * mono;
            let s0 = mono + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }

        let n = frames as f64;
        let rms = (sum_sq / n).sqrt();
        let power = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0);
        let amp_997 = (2.0 * power.sqrt()) / n;
        let ratio_997 = if rms > 0.0 {
            amp_997 / (2.0_f64.sqrt() * rms)
        } else {
            0.0
        };

        WindowMetrics {
            frames,
            peak_l,
            peak_r,
            peak_max: peak_l.max(peak_r),
            rms,
            amp_997,
            ratio_997,
        }
    }

    fn write_probe_wav(
        path: &Path,
        capture: &CaptureBuffer,
        total_frames: usize,
        sample_rate: u32,
    ) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("probe.wav ({}) の作成に失敗しました: {e}", path.display()))?;

        for frame in 0..total_frames {
            let (l, r) = capture.read_frame(frame);
            writer
                .write_sample(l)
                .map_err(|e| format!("probe.wav への書き込みに失敗しました: {e}"))?;
            writer
                .write_sample(r)
                .map_err(|e| format!("probe.wav への書き込みに失敗しました: {e}"))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("probe.wav の確定に失敗しました: {e}"))?;
        Ok(())
    }
}
