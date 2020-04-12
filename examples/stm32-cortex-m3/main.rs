#![no_std]
#![no_main]
// For allocator
#![feature(lang_items)]
#![feature(alloc_error_handler)]

// Remove this later and clean up code
#[allow(unused_variables)]
#[allow(unused_imports)]
#[allow(unused_assignments)]
#[allow(unused_attributes)]
pub mod hw;

// release profile: minimize the binary size of the application
// TODO: We want some custom panic handler that logs & resets
#[cfg(not(debug_assertions))]
extern crate panic_abort;
// dev profile: easier to debug panics; can put a breakpoint on `rust_begin_unwind`
//#[cfg(debug_assertions)]
extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics
//#[cfg(debug_assertions)]
//extern crate panic_semihosting;
// logs messages to the host stderr; requires a debugger

// requires nightly

use core::fmt::Write;
// extern crate panic_halt; // just stop the world
// extern crate panic_itm; // logs messages over ITM; requires ITM support
use core::ptr;

use cortex_m::asm;
use cortex_m_rt::exception;
use cortex_m_rt::{entry, ExceptionFrame};
use cortex_m_semihosting::hio::HStdout;
use cortex_m_semihosting::hio;
use alloc_cortex_m::CortexMHeap;
use crate::hw::VolatileStruct;
use freertos_rust::*;
use core::alloc::Layout;

const PERIPH_BASE: u32 = 0x40000000;
const AHBPERIPH_BASE: u32 = PERIPH_BASE + 0x20000;
const GPIOA_BASE: u32 = AHBPERIPH_BASE + 0x0000;
const RCC_BASE: u32 = AHBPERIPH_BASE + 0x3800;

fn set_led(gpio: &mut hw::GPIO, on: bool) {
    set_gpio(gpio, 1, !on)
}

fn set_gpio(gpio: &mut hw::GPIO, pin: u8, state: bool) {
    if state {
        gpio.bssrl |= 1 << pin;
    } else {
        gpio.bssrh |= 1 << pin;
    }
}


// this is the allocator the application will use
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

const HEAP_SIZE: usize = 512; // in bytes

// Setup IO for the LED and blink, does not return.
fn do_blink() {
    // Initialize LED
    let rcc = unsafe { hw::RCC::from_addr(RCC_BASE) };
    let gpio_a = unsafe { hw::GPIO::from_addr(GPIOA_BASE) };
    rcc.ahbenr.write(1);
    gpio_a.moder.write(4);

    loop {
        //println!("Hello from Task! {}", i);
        //CurrentTask::delay(Duration::ms(1000));
        //i = i + 1;

        let mut _i = 0;
        for _ in 0..2_00 {
            _i += 1;
        }
        set_led(gpio_a, true);
        for _ in 0..2_00 {
            _i += 1;
        }
        set_led(gpio_a, false);
    }
}

fn board_set_led(on: bool) {
    let rcc = unsafe { hw::RCC::from_addr(RCC_BASE) };
    let gpio_a = unsafe { hw::GPIO::from_addr(GPIOA_BASE) };
    rcc.ahbenr.write(1);
    gpio_a.moder.write(4);
    set_led(gpio_a, on);
}

#[entry]
fn main() -> ! {
    unsafe {
        // Permanently flash LED on assert
        FREERTOS_HOOKS.set_on_assert(|| { board_set_led(true); });
    }

    // Initialize the allocator BEFORE you use it
    unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) }

    // Just blink (works)
    // do_blink();

    // TODO: What comes now does not work yet!
    // Initialize Tasks and start FreeRTOS

    //println!("Starting FreeRTOS app ...");
    Task::new().name("hello").stack_size(128).priority(TaskPriority(1)).start(|| {
        // Blink forever
        // TODO: Replace loops with FreeRTOS vTaskDelay once this is running at all
        do_blink();
    }).unwrap();
    //println!("Task registered");
    //let free = freertos_rs_xPortGetFreeHeapSize();
    // println!("Free Memory: {}!", free);
    //println!("Starting scheduler");
    // TODO: Starting the scheduler fails, we need debugging to find the issue
    // Seems like we don't even get an assert
    FreeRtosUtils::start_scheduler();
    loop {
        //println!("Loop forever!");
    }
}

#[exception]
fn DefaultHandler(irqn: i16) {
    // custom default handler
    // irqn is negative for Cortex-M exceptions
    // irqn is positive for device specific (line IRQ)
    panic!("Exception: {}", irqn);
}


#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    board_set_led(true);

    if let Ok(mut hstdout) = hio::hstdout() {
        writeln!(hstdout, "{:#?}", ef).ok();
    }

    loop {}
}

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    board_set_led(true);
    asm::bkpt();
    loop {}
}