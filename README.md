# esp_pwm_reader
Wrapper around the mcpwm capture module for reading pwm signals.
# Documentation
- see <https://daschr.github.io/esp_pwm_reader/esp_pwm_reader/index.html>
  
# Usage
- you need to create a capture timer which can then be used by multiple channel readers
- f.e.
  ```
  use channel_reader::{CaptureTimer, ChannelReader};
  use esp_idf_hal::delay::FreeRtos;

  use esp_idf_sys::{
    gpio_num_t_GPIO_NUM_16, gpio_num_t_GPIO_NUM_17
  };

  fn main() {
    let capture_timer = CaptureTimer::new(0).unwrap();

    let channel1 = ChannelReader::new(&capture_timer, gpio_num_t_GPIO_NUM_16).unwrap();
    let channel2 = ChannelReader::new(&capture_timer, gpio_num_t_GPIO_NUM_17).unwrap();

    loop {
        println!(
            "ch1: {} ch2: {}",
            channel1.get_value(),
            channel2.get_value()
        );

        FreeRtos::delay_ms(11);
    }
  }
  ```
