#![doc(test(no_crate_inject))]

use esp_idf_sys::{
    __BindgenBitfieldUnit, gpio_num_t, mcpwm_cap_channel_handle_t, mcpwm_capture_channel_config_t,
    mcpwm_capture_timer_disable, mcpwm_capture_timer_stop, ESP_OK,
};
use esp_idf_sys::{
    esp_err_t, mcpwm_cap_channel_t, mcpwm_cap_timer_t, mcpwm_capture_channel_enable,
    mcpwm_capture_channel_register_event_callbacks, mcpwm_capture_edge_t_MCPWM_CAP_EDGE_POS,
    mcpwm_capture_event_callbacks_t, mcpwm_capture_event_data_t, mcpwm_capture_timer_config_t,
    mcpwm_capture_timer_enable, mcpwm_capture_timer_get_resolution, mcpwm_capture_timer_start,
    mcpwm_new_capture_channel, mcpwm_new_capture_timer,
    soc_periph_mcpwm_capture_clk_src_t_MCPWM_CAPTURE_CLK_SRC_DEFAULT, SOC_MCPWM_GROUPS,
};
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// This timer is used by a capture Channel insinde a ChannelReader.
/// It can be used by multiple ChannelReader instance.
/// Also see <https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/mcpwm.html?highlight=mcpwm_new_capture_timer#capture>
pub struct CaptureTimer {
    #[doc(hidden)]
    timer: *mut mcpwm_cap_timer_t,
}

/// Errors which can occur by calls to the mcpwm submodule of the esp-idf at different stages.
/// See mcpwm_capture_timer_enable ff.: <https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/mcpwm.html?highlight=mcpwm_new_capture_timer#_CPPv426mcpwm_capture_timer_enable24mcpwm_cap_timer_handle_t>
pub enum CaptureTimerError {
    InvalidGroupId,
    /// at mcpwm_capture_timer_start
    Start(esp_err_t),
    /// at mcpwm_capture_timer_enable
    Enable(esp_err_t),
    /// at mcpwm_capture_timer_stop
    Stop(esp_err_t),
    /// at mcpwm_capture_timer_disable
    Disable(esp_err_t),
    /// at mcpwm_new_capture_timer
    New(esp_err_t),
}

impl CaptureTimer {
    /// Create a new CaptureTimer, needs the MCPWM group id (see <https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/mcpwm.html?highlight=mcpwm_new_capture_timer#_CPPv4N20mcpwm_timer_config_t8group_idE>)
    pub fn new(capture_group_id: i32) -> Result<CaptureTimer, CaptureTimerError> {
        if capture_group_id < 0 || capture_group_id >= SOC_MCPWM_GROUPS as i32 {
            return Err(CaptureTimerError::InvalidGroupId);
        }

        let capture_timer_conf = mcpwm_capture_timer_config_t {
            group_id: capture_group_id,
            clk_src: soc_periph_mcpwm_capture_clk_src_t_MCPWM_CAPTURE_CLK_SRC_DEFAULT,
        };

        let mut capture_timer: *mut mcpwm_cap_timer_t = null_mut();

        let r = unsafe {
            mcpwm_new_capture_timer(
                &capture_timer_conf,
                (&mut capture_timer) as *mut *mut mcpwm_cap_timer_t,
            )
        };

        if r != ESP_OK {
            return Err(CaptureTimerError::New(r));
        }

        Ok(CaptureTimer {
            timer: capture_timer,
        })
    }

    /// Enable and start the timer, needed for reading a value from the ChannelReader
    pub fn enable_and_start(&self) -> Result<(), CaptureTimerError> {
        let mut r = unsafe { mcpwm_capture_timer_enable(self.timer) };
        if r != ESP_OK {
            return Err(CaptureTimerError::Enable(r));
        }

        r = unsafe { mcpwm_capture_timer_start(self.timer) };
        if r != ESP_OK {
            return Err(CaptureTimerError::Start(r));
        }

        Ok(())
    }

    /// Stopping and disabling the timer, all connected ChannelReader instances can then not measure the a pwm signal
    #[allow(unused)]
    pub fn stop_and_disable(&self) -> Result<(), CaptureTimerError> {
        let mut r = unsafe { mcpwm_capture_timer_stop(self.timer) };
        if r != ESP_OK {
            return Err(CaptureTimerError::Stop(r));
        }

        r = unsafe { mcpwm_capture_timer_disable(self.timer) };
        if r != ESP_OK {
            return Err(CaptureTimerError::Disable(r));
        }

        Ok(())
    }
}

#[doc(hidden)]
struct PwmCtx {
    start: AtomicU32,
    delta: AtomicU32,
}

/// Can read the current value of the PWM signal on the specified pin in microseconds
pub struct ChannelReader {
    #[doc(hidden)]
    ctx: Arc<PwmCtx>,
    #[doc(hidden)]
    capture_timer_res: u64,
}

/// Errors which can occur by calls to the mcpwm submodule of the esp-idf at different stages.
/// See mcpwm_new_capture_channel ff.: <https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/mcpwm.html?highlight=mcpwm_new_capture_timer#_CPPv425mcpwm_new_capture_channel24mcpwm_cap_timer_handle_tPK30mcpwm_capture_channel_config_tP26mcpwm_cap_channel_handle_t>
#[derive(Debug)]
pub enum ChannelReaderError {
    /// at mcpwm_new_capture_channel
    CreateChannel(esp_err_t),
    /// at mcpwm_capture_channel_enable
    EnableChannel(esp_err_t),
    /// at mcpwm_capture_channel_register_event_callbacks
    RegisterCallback(esp_err_t),
    /// at mcpwm_capture_timer_get_resolution
    GetTimerResolution(esp_err_t),
}

impl ChannelReader {
    /// Needs a CaptureTimer and the gpio input pin where the signal is present at
    pub fn new(
        capture_timer: &CaptureTimer,
        gpio_num: gpio_num_t,
    ) -> Result<Self, ChannelReaderError> {
        let mut bitfield = __BindgenBitfieldUnit::new([0u8; 1]);
        bitfield.set_bit(0, true);
        bitfield.set_bit(1, true);
        bitfield.set_bit(3, true);

        let mcpwm_capture_channel_config = mcpwm_capture_channel_config_t {
            gpio_num,
            prescale: 1,
            flags: esp_idf_sys::mcpwm_capture_channel_config_t__bindgen_ty_1 {
                _bitfield_align_1: [],
                _bitfield_1: bitfield,
                __bindgen_padding_0: [0u8; 3],
            },
        };

        let mut capture_channel: *mut mcpwm_cap_channel_t = null_mut();

        let mut r = unsafe {
            mcpwm_new_capture_channel(
                capture_timer.timer,
                &mcpwm_capture_channel_config,
                (&mut capture_channel) as *mut *mut mcpwm_cap_channel_t,
            )
        };
        if r != ESP_OK {
            return Err(ChannelReaderError::CreateChannel(r));
        }

        let mut capture_timer_res: u32 = 0;
        r = unsafe {
            mcpwm_capture_timer_get_resolution(
                capture_timer.timer,
                &mut capture_timer_res as *mut _,
            )
        };
        if r != ESP_OK {
            return Err(ChannelReaderError::GetTimerResolution(r));
        }

        let ctx = Arc::new(PwmCtx {
            start: AtomicU32::new(0),
            delta: AtomicU32::new(0),
        });

        let callbacks = mcpwm_capture_event_callbacks_t {
            on_cap: Some(Self::edge_event_handler),
        };

        r = unsafe {
            mcpwm_capture_channel_register_event_callbacks(
                capture_channel,
                &callbacks as *const _,
                (Arc::into_raw(ctx.clone()) as *const c_void) as *mut c_void,
            )
        };
        if r != ESP_OK {
            return Err(ChannelReaderError::RegisterCallback(r));
        }

        r = unsafe { mcpwm_capture_channel_enable(capture_channel) };
        if r != ESP_OK {
            return Err(ChannelReaderError::EnableChannel(r));
        }

        Ok(ChannelReader {
            ctx,
            capture_timer_res: capture_timer_res as u64,
        })
    }

    #[doc(hidden)]
    #[allow(unused_variables)]
    #[no_mangle]
    extern "C" fn edge_event_handler(
        channel_handle: mcpwm_cap_channel_handle_t,
        event_data: *const mcpwm_capture_event_data_t,
        ctx: *mut c_void,
    ) -> bool {
        let ctx: &PwmCtx = unsafe { &*(ctx as *mut _) };
        let event_data: &mcpwm_capture_event_data_t = unsafe { &*event_data };

        if event_data.cap_edge == mcpwm_capture_edge_t_MCPWM_CAP_EDGE_POS {
            ctx.start.store(event_data.cap_value, Ordering::Release);
        } else {
            let start = ctx.start.load(Ordering::Acquire);
            ctx.delta
                .store(event_data.cap_value - start, Ordering::Release);
        }

        true
    }

    /// Gets the current measured pwm signal in microseconds (μs)
    pub fn get_value(&self) -> u64 {
        let delta = self.ctx.delta.load(Ordering::Acquire) as u64;

        (delta * 1_000_000u64) / self.capture_timer_res
    }
}
