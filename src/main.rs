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
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    board::apply_clock_config(&mut config);

    let p = embassy_stm32::init(config);

    defmt::info!("custom-controller boot");

    uart::init(
        spawner,
        p.UART4,
        p.PA1,
        p.DMA1_CH2,
        p.USART2,
        p.PA3,
        p.DMA1_CH5,
        p.USART3,
        p.PC11,
        p.DMA1_CH1,
        p.USART6,
        p.PC7,
        p.DMA2_CH1,
        p.USART1,
        p.PA9,
        p.DMA2_CH7,
    )
    .await;

    adc::init(spawner, p.ADC1, p.DMA2_CH0, p.PB0, p.PA4, p.PA5);

	input::init(spawner, p.PC0, p.PC1, p.PC2, p.PC3, p.PB7);

    spawner.spawn(app::host_report_task()).unwrap();

    // M1/M2：先保证能启动、能跑 executor。
    // M3~M5：后续在这里按模块 spawn 各任务（UART/ADC/Buttons）。
    loop {
        embassy_time::Timer::after_millis(1000).await;
        defmt::info!("tick");
    }
}
