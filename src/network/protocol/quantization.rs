use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Масштаб квантизации: 32.0 = ~3cm precision
/// При масштабе 32: диапазон i16 (-32768..32767) покрывает ±1024 юнита
pub const QUANTIZATION_SCALE: f32 = 32.0;

/// Максимальный диапазон для квантизованных координат
pub const MAX_QUANTIZED_RANGE: f32 = 1024.0;

/// Квантизованный 2D вектор для компактной передачи по сети
/// Использует i16 вместо f32, уменьшая размер в 2 раза
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Vec2i16 {
    pub x: i16,
    pub y: i16,
}

impl Vec2i16 {
    /// Квантизует f32 вектор в i16 представление
    pub fn quantize(vec: Vec2) -> Self {
        Self {
            x: (vec.x * QUANTIZATION_SCALE)
                .round()
                .clamp(-32768.0, 32767.0) as i16,
            y: (vec.y * QUANTIZATION_SCALE)
                .round()
                .clamp(-32768.0, 32767.0) as i16,
        }
    }

    /// Деквантизует i16 обратно в f32 вектор
    pub fn dequantize(self) -> Vec2 {
        Vec2::new(
            self.x as f32 / QUANTIZATION_SCALE,
            self.y as f32 / QUANTIZATION_SCALE,
        )
    }

    /// Создает из сырых i16 значений
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    /// Нулевой вектор
    pub const ZERO: Self = Self { x: 0, y: 0 };
}

/// Trait для удобной квантизации Vec2 → Vec2i16
pub trait Quantize {
    fn to_i16(self) -> Vec2i16;
}

impl Quantize for Vec2 {
    fn to_i16(self) -> Vec2i16 {
        Vec2i16::quantize(self)
    }
}

/// Trait для удобной деквантизации Vec2i16 → Vec2
pub trait Dequantize {
    fn to_f32(self) -> Vec2;
}

impl Dequantize for Vec2i16 {
    fn to_f32(self) -> Vec2 {
        self.dequantize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_zero() {
        let vec = Vec2::ZERO;
        let quantized = Vec2i16::quantize(vec);
        assert_eq!(quantized, Vec2i16::ZERO);
        let dequantized = quantized.dequantize();
        assert_eq!(dequantized, Vec2::ZERO);
    }

    #[test]
    fn test_quantization_precision() {
        let vec = Vec2::new(10.0, 20.0);
        let quantized = Vec2i16::quantize(vec);
        let dequantized = quantized.dequantize();

        // Точность должна быть в пределах 1/QUANTIZATION_SCALE
        let epsilon = 1.0 / QUANTIZATION_SCALE;
        assert!((dequantized.x - vec.x).abs() < epsilon);
        assert!((dequantized.y - vec.y).abs() < epsilon);
    }

    #[test]
    fn test_quantization_range() {
        // Максимальное значение
        let vec = Vec2::new(1000.0, 1000.0);
        let quantized = Vec2i16::quantize(vec);
        let dequantized = quantized.dequantize();
        assert!((dequantized.x - vec.x).abs() < 0.1);

        // Отрицательное значение
        let vec = Vec2::new(-500.0, -500.0);
        let quantized = Vec2i16::quantize(vec);
        let dequantized = quantized.dequantize();
        assert!((dequantized.x - vec.x).abs() < 0.1);
    }

    #[test]
    fn test_quantization_clamping() {
        // Значение за пределами диапазона должно быть обрезано
        let vec = Vec2::new(10000.0, 10000.0);
        let quantized = Vec2i16::quantize(vec);
        assert!(quantized.x == i16::MAX);
        assert!(quantized.y == i16::MAX);
    }

    #[test]
    fn test_trait_usage() {
        let vec = Vec2::new(5.0, 10.0);
        let quantized: Vec2i16 = vec.to_i16();
        let dequantized: Vec2 = quantized.to_f32();

        let epsilon = 1.0 / QUANTIZATION_SCALE;
        assert!((dequantized.x - vec.x).abs() < epsilon);
        assert!((dequantized.y - vec.y).abs() < epsilon);
    }
}
