use crate::CalibData;
use core::time::Duration;

pub struct Calc {}

impl Calc {
    pub fn calc_heater_res(calib: &CalibData, amb_temp: i8, temp: u16) -> u8 {
        // cap temperature
        let temp = if temp <= 400 { temp } else { 400 };

        let var1 = amb_temp as i32 * calib.par_gh3 as i32 / 1000i32 * 256i32;
        let var2 = (calib.par_gh1 as i32 + 784i32)
            * (((calib.par_gh2 as i32 + 154009i32) * temp as i32 * 5i32 / 100i32 + 3276800i32)
                / 10i32);
        let var3 = var1 + var2 / 2i32;
        let var4 = var3 / (calib.res_heat_range as i32 + 4i32);
        let var5 = 131i32 * calib.res_heat_val as i32 + 65536i32;
        let heatr_res_x100 = (var4 / var5 - 250i32) * 34i32;
        ((heatr_res_x100 + 50i32) / 100i32) as u8
    }

    pub fn calc_heater_dur(duration: Duration) -> u8 {
        let mut factor: u8 = 0u8;
        // TODO replace once https://github.com/rust-lang/rust/pull/50167 has been merged
        const MILLIS_PER_SEC: u64 = 1_000;
        const NANOS_PER_MILLI: u64 = 1_000_000;
        let mut dur = (duration.as_secs() * MILLIS_PER_SEC)
            + (duration.subsec_nanos() as u64 / NANOS_PER_MILLI);
        if dur as i32 >= 0xfc0i32 {
            0xffu8 // Max duration
        } else {
            loop {
                if dur as i32 <= 0x3fi32 {
                    break;
                }
                dur = (dur as i32 / 4i32) as u64;
                factor = (factor as i32 + 1i32) as u8;
            }
            (dur as i32 + factor as i32 * 64i32) as u8
        }
    }

    ///
    /// * `calib` - Calibration data used during initalization
    /// * `temp_adc`
    /// * `temp_offset` - If set, the temperature t_fine will be increased by given
    ///                   value in celsius. Temperature offset in Celsius, e.g. 4, -8, 1.25
    pub fn calc_temperature(
        calib: &CalibData,
        temp_adc: u32,
        temp_offset: Option<f32>,
    ) -> (i16, i32) {
        let var1: i64 = (temp_adc as i64 >> 3) - ((calib.par_t1 as i64) << 1);
        let var2: i64 = (var1 * (calib.par_t2 as i64)) >> 11;
        let var3: i64 = ((var1 >> 1) * (var1 >> 1)) >> 12;
        let var3: i64 = (var3 * ((calib.par_t3 as i64) << 4)) >> 14;

        let temp_offset = match temp_offset {
            None => 0i32,
            Some(offset) if offset == 0.0 => 0i32,
            Some(offset) => {
                let signum: i32 = if offset.gt(&0.0) { 1 } else { -1 };
                signum * (((((offset * 100.0) as i32).abs() << 8) - 128) / 5)
            }
        };

        let t_fine: i32 = (var2 + var3) as i32 + temp_offset;
        let calc_temp: i16 = (((t_fine * 5) + 128) >> 8) as i16;
        (calc_temp, t_fine)
    }

    pub fn calc_pressure(calib: &CalibData, t_fine: i32, pres_adc: u32) -> u32 {
        let mut var1: i32 = (t_fine >> 1) - 64000;
        let mut var2: i32 = ((((var1 >> 2) * (var1 >> 2)) >> 11) * calib.par_p6 as i32) >> 2;
        var2 += (var1 * (calib.par_p5 as i32)) << 1;
        var2 = (var2 >> 2i32) + ((calib.par_p4 as i32) << 16i32);
        var1 = (((((var1 >> 2i32) * (var1 >> 2i32)) >> 13i32) * ((calib.par_p3 as i32) << 5i32))
            >> 3i32)
            + ((calib.par_p2 as i32 * var1) >> 1i32);
        var1 >>= 18i32;
        var1 = ((32768i32 + var1) * calib.par_p1 as i32) >> 15i32;
        let mut pressure_comp: i32 = 1048576u32.wrapping_sub(pres_adc) as i32;
        pressure_comp = ((pressure_comp - (var2 >> 12i32)) as u32).wrapping_mul(3125u32) as i32;
        if pressure_comp >= 0x40000000i32 {
            pressure_comp = ((pressure_comp as u32).wrapping_div(var1 as u32) << 1i32) as i32;
        } else {
            pressure_comp = ((pressure_comp << 1i32) as u32).wrapping_div(var1 as u32) as i32;
        }
        var1 = (calib.par_p9 as i32
            * (((pressure_comp >> 3i32) * (pressure_comp >> 3i32)) >> 13i32))
            >> 12i32;
        var2 = ((pressure_comp >> 2i32) * calib.par_p8 as i32) >> 13i32;
        let var3: i32 = ((pressure_comp >> 8i32)
            * (pressure_comp >> 8i32)
            * (pressure_comp >> 8i32)
            * calib.par_p10 as i32)
            >> 17i32;
        pressure_comp += (var1 + var2 + var3 + ((calib.par_p7 as i32) << 7i32)) >> 4i32;
        pressure_comp as u32
    }

    pub fn calc_humidity(calib: &CalibData, t_fine: i32, hum_adc: u16) -> u32 {
        let temp_scaled: i32 = (t_fine * 5i32 + 128i32) >> 8i32;
        let var1: i32 = hum_adc as i32
            - calib.par_h1 as i32 * 16i32
            - ((temp_scaled * calib.par_h3 as i32 / 100i32) >> 1i32);
        let var2: i32 = (calib.par_h2 as i32
            * (temp_scaled * calib.par_h4 as i32 / 100i32
                + ((temp_scaled * (temp_scaled * calib.par_h5 as i32 / 100i32)) >> 6i32) / 100i32
                + (1i32 << 14i32)))
            >> 10i32;
        let var3: i32 = var1 * var2;
        let var4: i32 = (calib.par_h6 as i32) << 7i32;
        let var4: i32 = (var4 + temp_scaled * calib.par_h7 as i32 / 100i32) >> 4i32;
        let var5: i32 = ((var3 >> 14i32) * (var3 >> 14i32)) >> 10i32;
        let var6: i32 = (var4 * var5) >> 1i32;
        let mut calc_hum: i32 = (((var3 + var6) >> 10i32) * 1000i32) >> 12i32;
        calc_hum = calc_hum.clamp(0i32, 100000i32);
        calc_hum as u32
    }

    pub fn calc_gas_resistance(calib: &CalibData, gas_res_adc: u16, gas_range: u8) -> u32 {
        let lookup_table1: [u32; 16] = [
            2147483647u32,
            2147483647u32,
            2147483647u32,
            2147483647u32,
            2147483647u32,
            2126008810u32,
            2147483647u32,
            2130303777u32,
            2147483647u32,
            2147483647u32,
            2143188679u32,
            2136746228u32,
            2147483647u32,
            2126008810u32,
            2147483647u32,
            2147483647u32,
        ];
        let lookup_table2: [u32; 16] = [
            4096000000u32,
            2048000000u32,
            1024000000u32,
            512000000u32,
            255744255u32,
            127110228u32,
            64000000u32,
            32258064u32,
            16016016u32,
            8000000u32,
            4000000u32,
            2000000u32,
            1,
            500000u32,
            250000u32,
            125000u32,
        ];
        let var1: i64 = ((1340 + 5 * calib.range_sw_err as i64)
            * lookup_table1[gas_range as usize] as i64)
            >> 16;
        let var2: u64 = (((gas_res_adc as i64) << 15) - 16777216 + var1) as u64;
        let var3: i64 = (lookup_table2[gas_range as usize] as i64 * var1) >> 9;
        let calc_gas_res: u32 = ((var3 + ((var2 as i64) >> 1i64)) / var2 as i64) as u32;
        calc_gas_res
    }
}
