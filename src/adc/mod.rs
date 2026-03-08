use embassy_executor::Spawner;
use embassy_stm32::peripherals;

pub mod joystick;

pub fn init(
	spawner: Spawner,
	adc1: peripherals::ADC1,
	dma: peripherals::DMA2_CH0,
	in8: peripherals::PB0,
	in4: peripherals::PA4,
	in5: peripherals::PA5,
) {
	spawner.spawn(joystick::joystick_adc_task(adc1, dma, in8, in4, in5)).unwrap();
}
