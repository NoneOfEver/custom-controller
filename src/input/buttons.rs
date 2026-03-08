//! 按键扫描与去抖骨架。
//!
//! 约束来自 HARDWARE.md：
//! - BUTTON1..4: PC0..PC3
//! - JOYSTICK_BUTTON: PB7
//! - 板级上拉，低有效
//! - MCU 侧 no-pull
//! - 周期扫描（不使用 EXTI）
//! - 去抖：20ms

//! ## 轮询式接口
//! - 后台任务 [`buttons_task`] 周期扫描并维护全局稳定态快照。
//! - 上层通过 [`latest`] 读取稳定态（而不是接收事件）。
//!
//! ## 去抖策略（最小实现）
//! - 每个按键维护一个 `Debouncer`：
//!   - `stable`：当前已确认的稳定状态
//!   - `candidate`：正在尝试确认的新状态
//!   - `count`：新状态连续出现的次数
//! - 只有当 `candidate` 连续出现达到阈值（20ms）才更新 `stable`。

//! ## Beginner Notes
//! - 任务循环里用 `Timer::after_millis(...).await` 形成周期：这等价于“延时并让出 CPU”。
//! - `Mutex` 里的值用 `Copy` 类型（小结构体）存放：读取/写入都是拷贝，避免借用生命周期难题。
//! - 这里选择 `u8` 计数器：阈值很小（10），节省空间且足够。

//! ## Reading Guide
//! - 先看 [`ButtonsSnapshot`]：它是上层读到的“稳定态快照”。
//! - 再看 [`latest`]：轮询接口。
//! - 最后看 [`buttons_task`]：任务如何扫描 GPIO、如何用 `Debouncer` 做 20ms 去抖。

use embassy_stm32::gpio::{Input, Pull};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
#[allow(dead_code)]
/// 按键 ID（预留：给未来协议/日志使用）。
pub enum ButtonId {
    Button1,
    Button2,
    Button3,
    Button4,
    Joystick,
}

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
/// 按键逻辑状态。
///
/// 说明：硬件为“低有效”，但在软件里统一转换为 Pressed/Released 逻辑态。
pub enum ButtonState {
    Released,
    Pressed,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format, Default)]
/// 5 个按键的稳定态快照。
///
/// 字段命名与 `HARDWARE.md` 的按键表保持一致。
pub struct ButtonsSnapshot {
    pub button1: ButtonState,
    pub button2: ButtonState,
    pub button3: ButtonState,
    pub button4: ButtonState,
    pub joystick: ButtonState,
}

static BUTTONS: Mutex<CriticalSectionRawMutex, ButtonsSnapshot> = Mutex::new(ButtonsSnapshot {
    button1: ButtonState::Released,
    button2: ButtonState::Released,
    button3: ButtonState::Released,
    button4: ButtonState::Released,
    joystick: ButtonState::Released,
});

#[allow(dead_code)]
/// 轮询读取按键“稳定态快照”。
///
/// - 如果后台任务尚未运行，初始值为全 Released。
/// - 读取会短暂进入互斥区并返回结构体副本。
pub async fn latest() -> ButtonsSnapshot {
    *BUTTONS.lock().await
}

fn level_to_state(is_low_active_pressed: bool) -> ButtonState {
    if is_low_active_pressed {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    }
}

#[derive(Copy, Clone)]
struct Debouncer {
    stable: ButtonState,
    candidate: ButtonState,
    count: u8,
}

impl Debouncer {
    fn new(initial: ButtonState) -> Self {
        Self {
            stable: initial,
            candidate: initial,
            count: 0,
        }
    }

    fn update<const THRESHOLD: u8>(&mut self, sample: ButtonState) -> Option<ButtonState> {
        // Rust 语法点：`const THRESHOLD: u8` 是“const generics”（常量泛型）。
        // 可以把它类比成 C++ 模板参数：Debouncer::update<10>(...)。
        // 好处是阈值在编译期常量化，逻辑分支更明确。
        if sample == self.stable {
            self.candidate = sample;
            self.count = 0;
            return None;
        }

        if sample != self.candidate {
            self.candidate = sample;
            self.count = 1;
            return None;
        }

        self.count = self.count.saturating_add(1);
        if self.count >= THRESHOLD {
            self.stable = self.candidate;
            self.count = 0;
            return Some(self.stable);
        }

        None
    }
}

#[embassy_executor::task]
/// 按键扫描 + 软件去抖后台任务。
///
/// # 周期与阈值
/// - 扫描周期：2ms
/// - 去抖时间：20ms
/// - 阈值：`THRESHOLD = DEBOUNCE_MS / SCAN_PERIOD_MS = 10`
///
/// # GPIO 配置
/// - `Pull::None`：因为硬件已做外部上拉
/// - `is_low() == true` 表示按下（低有效）
pub async fn buttons_task(
    button1: embassy_stm32::peripherals::PC0,
    button2: embassy_stm32::peripherals::PC1,
    button3: embassy_stm32::peripherals::PC2,
    button4: embassy_stm32::peripherals::PC3,
    joystick_button: embassy_stm32::peripherals::PB7,
) {
    // 周期扫描 + 20ms 去抖。
    const SCAN_PERIOD_MS: u64 = 2;
    const DEBOUNCE_MS: u64 = 20;
    const THRESHOLD: u8 = (DEBOUNCE_MS / SCAN_PERIOD_MS) as u8;
    const _: () = assert!(THRESHOLD > 0);

    let b1 = Input::new(button1, Pull::None);
    let b2 = Input::new(button2, Pull::None);
    let b3 = Input::new(button3, Pull::None);
    let b4 = Input::new(button4, Pull::None);
    let joy = Input::new(joystick_button, Pull::None);

    let mut d1 = Debouncer::new(ButtonState::Released);
    let mut d2 = Debouncer::new(ButtonState::Released);
    let mut d3 = Debouncer::new(ButtonState::Released);
    let mut d4 = Debouncer::new(ButtonState::Released);
    let mut dj = Debouncer::new(ButtonState::Released);

    loop {
        let s1 = level_to_state(b1.is_low());
        let s2 = level_to_state(b2.is_low());
        let s3 = level_to_state(b3.is_low());
        let s4 = level_to_state(b4.is_low());
        let sj = level_to_state(joy.is_low());

        let mut changed = false;
        let mut snapshot = *BUTTONS.lock().await;
        if let Some(v) = d1.update::<THRESHOLD>(s1) {
            snapshot.button1 = v;
            changed = true;
        }
        if let Some(v) = d2.update::<THRESHOLD>(s2) {
            snapshot.button2 = v;
            changed = true;
        }
        if let Some(v) = d3.update::<THRESHOLD>(s3) {
            snapshot.button3 = v;
            changed = true;
        }
        if let Some(v) = d4.update::<THRESHOLD>(s4) {
            snapshot.button4 = v;
            changed = true;
        }
        if let Some(v) = dj.update::<THRESHOLD>(sj) {
            snapshot.joystick = v;
            changed = true;
        }

        if changed {
            *BUTTONS.lock().await = snapshot;
        }

        embassy_time::Timer::after_millis(SCAN_PERIOD_MS).await;
    }
}
