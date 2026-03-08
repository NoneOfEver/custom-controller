//! 按键扫描与去抖骨架。
//!
//! 约束来自 HARDWARE.md：
//! - BUTTON1..4: PC0..PC3
//! - JOYSTICK_BUTTON: PB7
//! - 板级上拉，低有效
//! - MCU 侧 no-pull
//! - 周期扫描（不使用 EXTI）
//! - 去抖：20ms

use embassy_stm32::gpio::{Input, Pull};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
#[allow(dead_code)]
pub enum ButtonId {
    Button1,
    Button2,
    Button3,
    Button4,
    Joystick,
}

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
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
