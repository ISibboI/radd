/// Computes the dewpoint in degrees celsius.
///
/// Based on the formula for saturation vapor pressure from [UK NPL](https://www.npl.co.uk/resources/q-a/dew-point-and-relative-humidity).
///
/// `ln e_w(t) = ln 611.2 + (17.62 t)/(243.12+t)`
pub fn compute_dewpoint(
    temperature_celsius: f32,
    relative_humidity_percent: f32,
    _pressure_millibar: f32,
) -> f32 {
    let temperature_celsius = f64::from(temperature_celsius);
    let relative_humidity_ratio = f64::from(relative_humidity_percent) / 100.0;

    let saturation_vapor_pressure =
        (611.2f64.ln() + 17.62 * temperature_celsius / (243.12 + temperature_celsius)).exp();
    let actual_vapor_pressure = saturation_vapor_pressure * relative_humidity_ratio;
    let dewpoint_celsius =
        (1559.72 - 243.12 * actual_vapor_pressure.ln()) / (actual_vapor_pressure.ln() - 24.0354);

    dewpoint_celsius as f32
}
