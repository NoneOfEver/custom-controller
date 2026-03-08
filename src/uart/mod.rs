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

pub async fn init(
	spawner: Spawner,
	uart4: peripherals::UART4,
	uart4_rx: peripherals::PA1,
	uart4_rx_dma: peripherals::DMA1_CH2,
	usart2: peripherals::USART2,
	usart2_rx: peripherals::PA3,
	usart2_rx_dma: peripherals::DMA1_CH5,
	usart3: peripherals::USART3,
	usart3_rx: peripherals::PC11,
	usart3_rx_dma: peripherals::DMA1_CH1,
	usart6: peripherals::USART6,
	usart6_rx: peripherals::PC7,
	usart6_rx_dma: peripherals::DMA2_CH1,
	usart1: peripherals::USART1,
	usart1_tx: peripherals::PA9,
	usart1_tx_dma: peripherals::DMA2_CH7,
) {
	let mut cfg = usart::Config::default();
	cfg.baudrate = 9_600;
	cfg.data_bits = usart::DataBits::DataBits8;
	cfg.stop_bits = usart::StopBits::STOP1;
	cfg.parity = usart::Parity::ParityNone;

	// 4 路传感器 UART RX：DMA + IDLE 分帧
	let uart4_rx = usart::UartRx::new(uart4, UartIrqs, uart4_rx, uart4_rx_dma, cfg).unwrap();
	let usart2_rx = usart::UartRx::new(usart2, UartIrqs, usart2_rx, usart2_rx_dma, cfg).unwrap();
	let usart3_rx = usart::UartRx::new(usart3, UartIrqs, usart3_rx, usart3_rx_dma, cfg).unwrap();
	let usart6_rx = usart::UartRx::new(usart6, UartIrqs, usart6_rx, usart6_rx_dma, cfg).unwrap();

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
	host_tx::init(usart1, usart1_tx, usart1_tx_dma, cfg).await;
}
