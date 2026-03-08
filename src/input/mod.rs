use embassy_executor::Spawner;
use embassy_stm32::peripherals;

pub mod buttons;

pub fn init(
	spawner: Spawner,
	button1: peripherals::PC0,
	button2: peripherals::PC1,
	button3: peripherals::PC2,
	button4: peripherals::PC3,
	joystick_button: peripherals::PB7,
) {
	spawner
		.spawn(buttons::buttons_task(
			button1,
			button2,
			button3,
			button4,
			joystick_button,
		))
		.unwrap();
}
