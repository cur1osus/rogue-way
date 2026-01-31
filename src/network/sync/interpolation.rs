/// Реэкспорт существующих типов интерполяции из legacy модуля
/// Эти компоненты и системы уже работают, поэтому переиспользуем их
pub use crate::network::legacy::NetworkInterpolation;

// NOTE: ClientPredictionState и NetworkSnapshotBuffer приватные в legacy,
// поэтому не реэкспортируем их. Они будут заменены на новую реализацию
// в SnapshotBuffer и ClientTick.

// TODO: Адаптировать системы интерполяции для работы с новым протоколом
// - client_prediction_system
// - client_interpolation_tick_system
// Сейчас используем legacy версии
