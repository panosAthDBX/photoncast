//! Benchmarks for the calculator module.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use photoncast_calculator::{MathEvaluator, UnitConverter};

fn bench_math_evaluation(c: &mut Criterion) {
    let evaluator = MathEvaluator::new();

    c.bench_function("math_basic_arithmetic", |b| {
        b.iter(|| evaluator.evaluate(black_box("2 + 3 * 4")));
    });

    c.bench_function("math_with_parentheses", |b| {
        b.iter(|| evaluator.evaluate(black_box("(2 + 3) * (4 + 5)")));
    });

    c.bench_function("math_trigonometric", |b| {
        b.iter(|| evaluator.evaluate(black_box("sin(pi/2) + cos(0)")));
    });

    c.bench_function("math_complex_expression", |b| {
        b.iter(|| evaluator.evaluate(black_box("sqrt(pow(3, 2) + pow(4, 2)) * log(100)")));
    });

    c.bench_function("math_with_constants", |b| {
        b.iter(|| evaluator.evaluate(black_box("2 * pi * e")));
    });

    c.bench_function("math_factorial", |b| {
        b.iter(|| evaluator.evaluate(black_box("factorial(10)")));
    });
}

fn bench_unit_conversion(c: &mut Criterion) {
    let converter = UnitConverter::new();

    c.bench_function("unit_length_km_to_miles", |b| {
        b.iter(|| converter.convert(black_box(100.0), black_box("km"), black_box("miles")));
    });

    c.bench_function("unit_temperature_c_to_f", |b| {
        b.iter(|| converter.convert(black_box(100.0), black_box("c"), black_box("f")));
    });

    c.bench_function("unit_data_gb_to_mb", |b| {
        b.iter(|| converter.convert(black_box(1.0), black_box("gb"), black_box("mb")));
    });

    c.bench_function("unit_weight_kg_to_lb", |b| {
        b.iter(|| converter.convert(black_box(75.0), black_box("kg"), black_box("lb")));
    });

    c.bench_function("unit_speed_kmh_to_mph", |b| {
        b.iter(|| converter.convert(black_box(120.0), black_box("km/h"), black_box("mph")));
    });
}

fn bench_expression_parsing(c: &mut Criterion) {
    use photoncast_calculator::ExpressionParser;

    let parser = ExpressionParser::new();

    c.bench_function("parse_math_expression", |b| {
        b.iter(|| parser.parse(black_box("2 + 3 * 4")));
    });

    c.bench_function("parse_currency_expression", |b| {
        b.iter(|| parser.parse(black_box("100 usd to eur")));
    });

    c.bench_function("parse_unit_expression", |b| {
        b.iter(|| parser.parse(black_box("5 km to miles")));
    });

    c.bench_function("parse_percentage_expression", |b| {
        b.iter(|| parser.parse(black_box("32% of 500")));
    });

    c.bench_function("parse_datetime_expression", |b| {
        b.iter(|| parser.parse(black_box("days until dec 25")));
    });
}

fn bench_datetime_calculations(c: &mut Criterion) {
    use photoncast_calculator::DateTimeCalculator;

    let calculator = DateTimeCalculator::new();

    c.bench_function("datetime_time_in_city", |b| {
        b.iter(|| calculator.evaluate(black_box("time in tokyo")));
    });

    c.bench_function("datetime_days_until", |b| {
        b.iter(|| calculator.evaluate(black_box("days until dec 25")));
    });

    c.bench_function("datetime_relative_date", |b| {
        b.iter(|| calculator.evaluate(black_box("35 days ago")));
    });

    c.bench_function("datetime_time_conversion", |b| {
        b.iter(|| calculator.evaluate(black_box("5pm ldn in sf")));
    });
}

criterion_group!(
    benches,
    bench_math_evaluation,
    bench_unit_conversion,
    bench_expression_parsing,
    bench_datetime_calculations,
);

criterion_main!(benches);
