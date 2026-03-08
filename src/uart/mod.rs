use embassy_executor::Spawner;
use embassy_stm32::{bind_interrupts, peripherals, usart};

pub mod host_tx;
pub mod sensor_ports;

bind_interrupts!(pub struct UartIrqs {
	UART4 => usart::InterruptHandler<peripherals::UART4>;
	USART2 => usart::InterruptHandler<peripherals::USART2>;
	USART3 => usart::InterruptHandler<peripherals::USART3>;
	USART6 => usart::InterruptHandler<peripherals::USART6>;
});

pub async fn init(spawner: Spawner, p: embassy_stm32::Peripherals) {
	let mut cfg = usart::Config::default();
	cfg.baudrate = 9_600;
	cfg.data_bits = usart::DataBits::DataBits8;
	cfg.stop_bits = usart::StopBits::STOP1;
	cfg.parity = usart::Parity::ParityNone;

	// 4 路传感器 UART RX：DMA + IDLE 分帧
	let uart4_rx = usart::UartRx::new(p.UART4, UartIrqs, p.PA1, p.DMA1_CH2, cfg).unwrap();
	let usart2_rx = usart::UartRx::new(p.USART2, UartIrqs, p.PA3, p.DMA1_CH5, cfg).unwrap();
	let usart3_rx = usart::UartRx::new(p.USART3, UartIrqs, p.PC11, p.DMA1_CH1, cfg).unwrap();
	let usart6_rx = usart::UartRx::new(p.USART6, UartIrqs, p.PC7, p.DMA2_CH1, cfg).unwrap();

	spawner
		.spawn(sensor_ports::sensor_rx_task(
			uart4_rx,
			sensor_ports::SensorPortId::Uart4,
		))
		.unwrap();
	spawner
		.spawn(sensor_ports::sensor_rx_task(
			usart2_rx,
			sensor_ports::SensorPortId::Usart2,
		))
		.unwrap();
	spawner
		.spawn(sensor_ports::sensor_rx_task(
			usart3_rx,
			sensor_ports::SensorPortId::Usart3,
		))
		.unwrap();
	spawner
		.spawn(sensor_ports::sensor_rx_task(
			usart6_rx,
			sensor_ports::SensorPortId::Usart6,
		))
		.unwrap();

	// 上位机链路：USART1 → RS485（TX-only）
	host_tx::init(p.USART1, p.PA9, p.DMA2_CH7, cfg).await;
}
