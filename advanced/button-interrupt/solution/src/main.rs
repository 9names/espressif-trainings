// reference:
// https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/system/freertos.html

use std::{ffi::CString, ptr, sync::mpsc};

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported (`self as _`)
use esp_idf_sys::{
    self as _, c_types::c_void, esp, gpio_config, gpio_config_t, gpio_install_isr_service,
    gpio_int_type_t_GPIO_INTR_POSEDGE, gpio_isr_handler_add, gpio_mode_t_GPIO_MODE_INPUT,
    xQueueGenericCreate, xQueueGiveFromISR, xQueueReceive, xTaskCreatePinnedToCore, QueueHandle_t,
};

static mut EVENT_QUEUE: Option<QueueHandle_t> = None;
const PORT_MAX_DELAY: u32 = 0xffffffff;

// TODO place code in ram using IRAM linker feature
// https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/linker-script-generation.html
// uncommenting this "works", TODO test/verify
#[link_section = ".iram0.text"]
unsafe extern "C" fn button_interrupt(_: *mut c_void) {
    xQueueGiveFromISR(EVENT_QUEUE.unwrap(), std::ptr::null_mut());
}

fn main() -> anyhow::Result<()> {
    const GPIO_NUM: i32 = 9;
    
    // Configures the button
    let io_conf = gpio_config_t {
        pin_bit_mask: 1 << GPIO_NUM,
        mode: gpio_mode_t_GPIO_MODE_INPUT,
        pull_up_en: true.into(),
        pull_down_en: false.into(),
        intr_type: gpio_int_type_t_GPIO_INTR_POSEDGE, // positive edge trigger = button down
    };

    // Queue configurations
    const QUEUE_TYPE_BASE: u8 = 0;
    const ITEM_SIZE: u32 = 0; // we're not posting any actual data, just notifying
    const QUEUE_SIZE: u32 = 2; // 1 might be enough?

    unsafe {
        // Writes button configuration into register
        esp!(gpio_config(&io_conf))?;
        
        // Instantiates the queue
        EVENT_QUEUE = Some(xQueueGenericCreate(QUEUE_SIZE, ITEM_SIZE, QUEUE_TYPE_BASE));
        
        // Sets the flag that the interrupt is handled in RAM
        const ESP_INTR_FLAG_IRAM: i32 = 1 << 10; // ISR will be executed even when caches are disabled
        
        // Installs the interrupt handler
        esp!(gpio_install_isr_service(ESP_INTR_FLAG_IRAM))?;

        // Adds the GPIO to the interrupt handler and the function that is to be executed upon firing of the interrupt
        esp!(gpio_isr_handler_add(
            GPIO_NUM,
            Some(button_interrupt),
            std::ptr::null_mut()
        ))?;
    }

    // Reads the queue in a loop.
    loop {
        unsafe {
            let res = xQueueReceive(EVENT_QUEUE.unwrap(), ptr::null_mut(), PORT_MAX_DELAY);
            // If the item in the queue is > 0, the button is pressed. 
            match res {
                _ => println!("button pressed!"),
                0 => {}
            };
        }
    }
}
