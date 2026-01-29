use bevy::prelude::*;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Структура для постоянных улучшений игрока
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermanentUpgrades {
    // Улучшения базовых статов (§3.2.2)
    pub max_hp_level: u32,         // +10 HP за уровень
    pub movement_speed_level: u32, // +5% за уровень
    pub starting_xp_level: u32,    // Начинать с более высоким уровнем
    pub pet_slots: u32,            // Количество активных слотов питомцев (базово 1)

    // Экономические улучшения (§3.2.3)
    pub gold_multiplier_level: u32, // +10% золота за уровень
    pub luck_level: u32,            // +5% качество выбора за уровень
    pub starting_gold_level: u32,   // Начинать с бонусным золотом
}

impl PermanentUpgrades {
    /// Вычисляет итоговое значение max HP
    pub fn get_max_hp(&self) -> f32 {
        100.0 + (self.max_hp_level as f32 * 10.0)
    }

    /// Вычисляет множитель скорости движения
    pub fn get_movement_speed_multiplier(&self) -> f32 {
        1.0 + (self.movement_speed_level as f32 * 0.05)
    }

    /// Вычисляет стартовый уровень
    pub fn get_starting_level(&self) -> u32 {
        1 + self.starting_xp_level
    }

    /// Вычисляет множитель золота
    pub fn get_gold_multiplier(&self) -> f32 {
        1.0 + (self.gold_multiplier_level as f32 * 0.1)
    }

    /// Вычисляет шанс удачи (0.0 - 1.0)
    #[allow(dead_code)]
    pub fn get_luck(&self) -> f32 {
        (self.luck_level as f32 * 0.05).min(1.0)
    }

    /// Вычисляет стартовое золото
    pub fn get_starting_gold(&self) -> u32 {
        self.starting_gold_level * 50
    }

    /// Получить общее количество слотов питомцев
    #[allow(dead_code)]
    pub fn get_total_pet_slots(&self) -> u32 {
        1 + self.pet_slots
    }
}

/// Статистика игрока
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Statistics {
    pub total_kills: u32,
    pub total_gold_earned: u32,
    pub total_xp_earned: u32,
    pub total_damage_dealt: u32,
    pub total_damage_taken: u32,
}

/// Основная структура данных сохранения (§4.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub total_gold: u32,
    pub total_runs: u32,
    pub best_time: u32, // в секундах
    pub unlocked_pets: Vec<String>,
    pub permanent_upgrades: PermanentUpgrades,
    pub achievements: Vec<String>,
    pub statistics: Statistics,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            total_gold: 0,
            total_runs: 0,
            best_time: 0,
            unlocked_pets: vec![
                "guard_dog".to_string(),
                "fire_sprite".to_string(), // Стартовые разблокированные питомцы
            ],
            permanent_upgrades: PermanentUpgrades::default(),
            achievements: Vec::new(),
            statistics: Statistics::default(),
        }
    }
}

impl SaveData {
    /// Получить путь к файлу сохранения
    fn get_save_path() -> PathBuf {
        // Используем стандартную директорию для данных приложения
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("Roggy");

        // Создаем директорию если её нет
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }

        path.push("save.json");
        path
    }

    /// Сохранить данные в файл
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_save_path();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Ошибка сериализации: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Ошибка записи файла: {}", e))?;

        Ok(())
    }

    /// Загрузить данные из файла
    pub fn load() -> Self {
        let path = Self::get_save_path();

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<SaveData>(&contents) {
                Ok(data) => data,
                Err(_e) => Self::default(),
            },
            Err(_e) => Self::default(),
        }
    }

    /// Добавить золото
    pub fn add_gold(&mut self, amount: u32) {
        self.total_gold += amount;
        self.statistics.total_gold_earned += amount;
    }

    /// Потратить золото (возвращает true если хватило)
    pub fn spend_gold(&mut self, amount: u32) -> bool {
        if self.total_gold >= amount {
            self.total_gold -= amount;
            true
        } else {
            false
        }
    }

    /// Разблокировать питомца
    pub fn unlock_pet(&mut self, pet_id: &str) {
        if !self.unlocked_pets.contains(&pet_id.to_string()) {
            self.unlocked_pets.push(pet_id.to_string());
        }
    }

    /// Проверить разблокирован ли питомец
    pub fn is_pet_unlocked(&self, pet_id: &str) -> bool {
        self.unlocked_pets.contains(&pet_id.to_string())
    }

    /// Разблокировать достижение
    #[allow(dead_code)]
    pub fn unlock_achievement(&mut self, achievement_id: &str) {
        if !self.achievements.contains(&achievement_id.to_string()) {
            self.achievements.push(achievement_id.to_string());
        }
    }
}

/// Ресурс для управления мета-прогрессией игры
#[derive(Resource, Debug, Clone)]
pub struct MetaProgression {
    pub save_data: SaveData,
    pub should_save: bool, // Флаг для отложенного сохранения
}

impl Default for MetaProgression {
    fn default() -> Self {
        Self {
            save_data: SaveData::load(),
            should_save: false,
        }
    }
}

impl MetaProgression {
    /// Создать новый ресурс с загрузкой данных
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Отметить что нужно сохранить данные
    pub fn mark_dirty(&mut self) {
        self.should_save = true;
    }

    /// Сохранить данные если помечены как измененные
    pub fn save_if_dirty(&mut self) {
        if self.should_save {
            if let Err(e) = self.save_data.save() {
                let _ = e;
            } else {
                self.should_save = false;
            }
        }
    }
}

/// Система для автоматического сохранения
pub fn auto_save_system(mut meta: ResMut<MetaProgression>) {
    meta.save_if_dirty();
}

/// Система для сохранения при выходе
#[allow(dead_code)]
pub fn save_on_exit_system(meta: Res<MetaProgression>) {
    if let Err(e) = meta.save_data.save() {
        let _ = e;
    }
}
