use serde_json::{Value, json};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tyme4rs::tyme::Tyme;
use tyme4rs::tyme::lunar::{LunarDay, LunarYear};
use tyme4rs::tyme::solar::{SolarDay, SolarMonth, SolarTerm};

/// 提取农历年月日，闰月保留后端负月号。
fn lunar_fields(day: &LunarDay) -> [i32; 3] {
    let month = day.get_lunar_month();
    [
        month.get_lunar_year().get_year() as i32,
        month.get_month_with_leap() as i32,
        day.get_day() as i32,
    ]
}

/// 提取后端民用年月日供跨实现比较。
fn solar_fields(day: &SolarDay) -> [i32; 3] {
    [
        day.get_year() as i32,
        day.get_month() as i32,
        day.get_day() as i32,
    ]
}

/// 对输入参考日期执行公农历往返，输出可与独立来源比较的 JSON。
fn calendar(input: &str) -> Value {
    let rows: Vec<[i32; 6]> = serde_json::from_str(input).expect("read calendar references");
    let output: Vec<_> = rows.iter().map(|row| {
        let [y, m, d, ..] = *row;
        let solar = SolarDay::from_ymd(y as isize, m as usize, d as usize);
        let lunar = solar.get_lunar_day();
        json!({"date": [y,m,d], "lunar": lunar_fields(&lunar), "back": solar_fields(&lunar.get_solar_day())})
    }).collect();
    json!(output)
}

/// 提取指定年度节气的精确时刻及所属日，按小寒起序。
fn terms(input: &str) -> Value {
    let rows: Vec<[i32; 2]> = serde_json::from_str(input).expect("read term references");
    let output: Vec<_> = rows.iter().map(|[year, index]| {
        let term = SolarTerm::from_index(*year as isize, 1).next(*index as isize);
        let exact = term.get_julian_day().get_solar_time();
        json!({
            "year":year, "index":index,
            "actual":[exact.get_year(),exact.get_month(),exact.get_day(),exact.get_hour(),exact.get_minute(),exact.get_second()],
            "calendar_day":solar_fields(&term.get_solar_day()),
        })
    }).collect();
    json!(output)
}

/// 检查单个后端民用日期经农历往返是否仍为原日。
fn check_solar(day: SolarDay) -> Result<(), String> {
    let lunar = day.get_lunar_day();
    let back = lunar.get_solar_day();
    if solar_fields(&day) != solar_fields(&back) {
        return Err(format!("{day} => {lunar} => {back}"));
    }
    Ok(())
}

/// 扫描后端边界和元数据异常，收集不一致及 panic 供适配层确定排除范围。
fn boundaries() -> Value {
    // 此处主动探测上游越界行为，不把捕获 panic 当作应用层错误处理策略。
    std::panic::set_hook(Box::new(|_| {}));
    let mut solar_checks = 0usize;
    let mut lunar_checks = 0usize;
    let mut solar_daily_checks = 0usize;
    let mut lunar_daily_checks = 0usize;
    let mut boundary_panics = Vec::new();
    let mut issues = Vec::new();
    let mut month_enumeration_panics = Vec::new();
    let mut lunar_year_metadata_panics = Vec::new();
    let mut lunar_month_metadata_panics = Vec::new();
    for year in 1..=9999 {
        for month in 1..=12 {
            let metadata = SolarMonth::from_ym(year, month);
            if catch_unwind(AssertUnwindSafe(|| metadata.get_days())).is_err() {
                month_enumeration_panics.push([year, month as isize]);
            }
            let first = SolarDay::from_ymd(year, month, 1);
            let count = metadata.get_day_count();
            for offset in 0..count {
                let day = first.next(offset as isize);
                solar_daily_checks += 1;
                if offset == 0 || offset + 1 == count {
                    solar_checks += 1;
                }
                match catch_unwind(AssertUnwindSafe(|| check_solar(day))) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => issues.push(
                        json!({"kind":"solar_roundtrip","date":solar_fields(&day),"error":error}),
                    ),
                    Err(_) => issues.push(json!({"kind":"solar_panic","date":solar_fields(&day)})),
                }
            }
        }
    }
    for year in (1..=9999).chain([-1, 0]) {
        let months =
            match catch_unwind(AssertUnwindSafe(|| LunarYear::from_year(year).get_months())) {
                Ok(months) => months,
                Err(_) => {
                    lunar_year_metadata_panics.push(year);
                    continue;
                }
            };
        for month in months {
            let number = month.get_month_with_leap();
            let day_count = match catch_unwind(AssertUnwindSafe(|| month.get_day_count())) {
                Ok(count) => count,
                Err(_) => {
                    lunar_month_metadata_panics.push([year, number]);
                    continue;
                }
            };
            for day in 1..=day_count {
                lunar_daily_checks += 1;
                if day == 1 || day == day_count {
                    lunar_checks += 1;
                }
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let lunar = LunarDay::from_ymd(year, number, day);
                    let solar = lunar.get_solar_day();
                    let back = solar.get_lunar_day();
                    (
                        lunar_fields(&lunar),
                        solar_fields(&solar),
                        lunar_fields(&back),
                    )
                }));
                match result {
                    Ok((a, solar, b)) if a != b => issues
                        .push(json!({"kind":"lunar_roundtrip","date":a,"solar":solar,"back":b})),
                    Ok(_) => {}
                    Err(_) if year <= 0 || year == 9999 => {
                        boundary_panics.push(json!([year, number, day]))
                    }
                    Err(_) => issues.push(json!({"kind":"lunar_panic","date":[year,number,day]})),
                }
            }
        }
    }
    let endpoints: Vec<_> = [(1, 1, 1), (9999, 12, 31)]
        .into_iter()
        .map(|(y, m, d)| {
            let solar = SolarDay::from_ymd(y, m, d);
            json!({"solar":solar_fields(&solar),"lunar":lunar_fields(&solar.get_lunar_day())})
        })
        .collect();
    let probes: Vec<_> = [
        (0, 1, 1),
        (10000, 1, 1),
        (1500, 2, 29),
        (1582, 10, 4),
        (1582, 10, 5),
        (1582, 10, 14),
        (1582, 10, 15),
    ]
    .into_iter()
    .map(|(y, m, d)| {
        let result = catch_unwind(|| SolarDay::new(y, m, d));
        let status = match result {
            Ok(Ok(_)) => "accepted",
            Ok(Err(_)) => "rejected",
            Err(_) => "panic",
        };
        json!({"date":[y,m,d],"status":status})
    })
    .collect();
    json!({"solar_daily_checks":solar_daily_checks,"lunar_daily_checks":lunar_daily_checks,"solar_month_boundary_checks":solar_checks,"lunar_month_boundary_checks":lunar_checks,"month_enumeration_panics":month_enumeration_panics,"lunar_year_metadata_panics":lunar_year_metadata_panics,"lunar_month_metadata_panics":lunar_month_metadata_panics,"boundary_panics":boundary_panics,"issues":issues,"endpoints":endpoints,"probes":probes})
}

/// 按指定验证模式读取输入并写出报告，工具失败直接终止本次验证。
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "usage: MODE INPUT OUTPUT");
    let result = match args[1].as_str() {
        "calendar" => calendar(&fs::read_to_string(&args[2]).expect("read input")),
        "terms" => terms(&fs::read_to_string(&args[2]).expect("read input")),
        "boundaries" => boundaries(),
        _ => panic!("unknown validation mode"),
    };
    fs::write(
        &args[3],
        serde_json::to_vec(&result).expect("serialize results"),
    )
    .expect("write results");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证闰月负号在转换中保留。
    fn signed_leap_month_survives_conversion() {
        assert_eq!(
            lunar_fields(&SolarDay::from_ymd(2023, 3, 22).get_lunar_day()),
            [2023, -2, 1]
        );
    }

    #[test]
    /// 验证民用日期下端点对应的农历年零不丢失。
    fn lunar_year_zero_is_preserved_at_the_lower_solar_endpoint() {
        assert_eq!(
            lunar_fields(&SolarDay::from_ymd(1, 1, 1).get_lunar_day()),
            [0, 11, 18]
        );
    }

    #[test]
    /// 验证节气列表从小寒而非前一年冬至起序。
    fn start_with_xiaohan_not_previous_winter_solstice() {
        let value = terms("[[2024,0],[2024,23]]");
        assert_eq!(value[0]["actual"][1], 1);
        assert_eq!(value[1]["actual"][1], 12);
    }
}
