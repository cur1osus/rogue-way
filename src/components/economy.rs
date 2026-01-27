use bevy::prelude::*;

/// Компонент опыта игрока
#[derive(Component)]
pub struct Experience {
    pub current: u32,
    pub to_next_level: u32,
    pub level: u32,
}

impl Default for Experience {
    fn default() -> Self {
        Self {
            current: 0,
            to_next_level: 100, // Базовое количество XP для первого уровня
            level: 1,
        }
    }
}

impl Experience {
    /// Создать опыт с заданным уровнем (для постоянных улучшений)
    pub fn new_with_level(level: u32) -> Self {
        let mut exp = Self::default();

        // Поднимаем уровень до нужного
        for _ in 1..level {
            exp.level += 1;
            exp.to_next_level = (exp.to_next_level as f32 * 1.1) as u32;
        }

        exp
    }

    /// Добавить опыт и проверить повышение уровня
    pub fn add_xp(&mut self, amount: u32) -> bool {
        self.current += amount;

        if self.current >= self.to_next_level {
            self.level_up();
            return true;
        }

        false
    }

    /// Повысить уровень
    fn level_up(&mut self) {
        self.current -= self.to_next_level;
        self.level += 1;

        // Увеличиваем требование XP для следующего уровня (+ 10% каждый уровень)
        self.to_next_level = (self.to_next_level as f32 * 1.1) as u32;
    }
}

/// Гем опыта (дроп с врагов)
#[derive(Component)]
pub struct XpGem {
    pub value: u32,
}

/// Компонент золота игрока
#[derive(Component)]
pub struct Gold {
    pub amount: u32,
}

impl Default for Gold {
    fn default() -> Self {
        Self { amount: 0 }
    }
}

impl Gold {
    /// Создать золото с заданным количеством (для постоянных улучшений)
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }

    pub fn add(&mut self, amount: u32) {
        self.amount += amount;
    }
}

/// Монета золота (дроп с врагов)
#[derive(Component)]
pub struct GoldPickup {
    pub value: u32,
}

/// Таймер для подсветки золота (начинает сверкать через 3 секунды)
#[derive(Component)]
pub struct GoldHighlightTimer {
    pub timer: Timer,
    pub is_highlighted: bool,
}

/// Компонент анимации сверкания золота
#[derive(Component)]
pub struct GoldAnimation {
    pub timer: Timer,
    pub current_frame: usize,
}
