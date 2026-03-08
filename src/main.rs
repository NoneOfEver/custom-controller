#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::Config;

mod adc;
mod app;
mod board;
mod input;
mod protocol;
mod uart;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    board::apply_clock_config(&mut config);

    let p = embassy_stm32::init(config);

    defmt::info!("custom-controller boot");

    uart::init(_spawner, p).await;

    // M1/M2：先保证能启动、能跑 executor。
    // M3~M5：后续在这里按模块 spawn 各任务（UART/ADC/Buttons）。
    loop {
        embassy_time::Timer::after_millis(1000).await;
        defmt::info!("tick");
    }
}
