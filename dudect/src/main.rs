use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

const INPUT_LEN: usize = 64;
const OUTPUT_LEN: usize = 48;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Clone, Copy)]
struct Config {
    samples: usize,
    iterations: usize,
    threshold: f64,
    warmup: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            samples: 20_000,
            iterations: 64,
            threshold: 10.0,
            warmup: 1_000,
        }
    }
}

#[derive(Clone, Copy)]
struct Accumulator {
    count: usize,
    mean: f64,
    m2: f64,
}

impl Accumulator {
    const fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    fn push(&mut self, value: f64) {
        self.count += 1;
        let count = self.count as f64;
        let delta = value - self.mean;
        self.mean += delta / count;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    fn variance(self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dudect: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let config = parse_args()?;
    validate_config(config)?;

    let mut rng = XorShift64::new(0x6261_7365_3634_6e67);
    let fixed = [b'A'; INPUT_LEN];
    let mut random = [0u8; INPUT_LEN];
    let mut output = [0u8; OUTPUT_LEN];

    for _ in 0..config.warmup {
        fill_random_base64(&mut random, &mut rng);
        let class = rng.next() & 1;
        let input = if class == 0 { &fixed } else { &random };
        measure_decode(input, &mut output, config.iterations)?;
    }

    let mut fixed_stats = Accumulator::new();
    let mut random_stats = Accumulator::new();

    for _ in 0..config.samples {
        fill_random_base64(&mut random, &mut rng);
        let class = rng.next() & 1;
        let input = if class == 0 { &fixed } else { &random };
        let elapsed = measure_decode(input, &mut output, config.iterations)?;

        if class == 0 {
            fixed_stats.push(elapsed);
        } else {
            random_stats.push(elapsed);
        }
    }

    if fixed_stats.count < 2 || random_stats.count < 2 {
        return Err("both timing classes need at least two samples".to_owned());
    }

    let t = welch_t(fixed_stats, random_stats);
    println!(
        "dudect: samples={} iterations={} fixed_n={} random_n={} fixed_mean_ns={:.3} random_mean_ns={:.3} t={:.3} threshold={:.3}",
        config.samples,
        config.iterations,
        fixed_stats.count,
        random_stats.count,
        fixed_stats.mean,
        random_stats.mean,
        t,
        config.threshold
    );

    if t.abs() > config.threshold {
        Err(format!(
            "absolute Welch t-statistic {:.3} exceeded threshold {:.3}",
            t.abs(),
            config.threshold
        ))
    } else {
        Ok(())
    }
}

fn measure_decode(
    input: &[u8; INPUT_LEN],
    output: &mut [u8; OUTPUT_LEN],
    iterations: usize,
) -> Result<f64, String> {
    let start = Instant::now();
    for _ in 0..iterations {
        let written = base64_ng::ct::STANDARD_NO_PAD
            .decode_slice_clear_tail(black_box(input), black_box(output))
            .map_err(|error| format!("constant-time decode failed: {error}"))?;
        black_box(written);
        black_box(&*output);
    }
    let nanos = start.elapsed().as_nanos() as f64;
    Ok(nanos / iterations as f64)
}

fn fill_random_base64(output: &mut [u8; INPUT_LEN], rng: &mut XorShift64) {
    for byte in output {
        let index = (rng.next() & 63) as usize;
        *byte = ALPHABET[index];
    }
}

fn welch_t(left: Accumulator, right: Accumulator) -> f64 {
    let left_variance = left.variance();
    let right_variance = right.variance();
    let denominator =
        (left_variance / left.count as f64 + right_variance / right.count as f64).sqrt();

    if denominator == 0.0 {
        0.0
    } else {
        (left.mean - right.mean) / denominator
    }
}

fn parse_args() -> Result<Config, String> {
    let mut config = Config::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--samples" => config.samples = parse_usize(&arg, args.next())?,
            "--iters" | "--iterations" => config.iterations = parse_usize(&arg, args.next())?,
            "--threshold" => config.threshold = parse_f64(&arg, args.next())?,
            "--warmup" => config.warmup = parse_usize(&arg, args.next())?,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument `{arg}`")),
        }
    }

    Ok(config)
}

fn parse_usize(flag: &str, value: Option<String>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse()
        .map_err(|_| format!("{flag} requires a positive integer"))
}

fn parse_f64(flag: &str, value: Option<String>) -> Result<f64, String> {
    value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse()
        .map_err(|_| format!("{flag} requires a number"))
}

fn validate_config(config: Config) -> Result<(), String> {
    if config.samples < 4 {
        return Err("--samples must be at least 4".to_owned());
    }
    if config.iterations == 0 {
        return Err("--iters must be at least 1".to_owned());
    }
    if !(config.threshold.is_finite() && config.threshold > 0.0) {
        return Err("--threshold must be a positive finite number".to_owned());
    }
    Ok(())
}

fn print_help() {
    println!(
        "Usage: base64-ng-dudect [--samples N] [--iters N] [--threshold T] [--warmup N]\n\
         \n\
         Measures fixed-vs-random valid Base64 inputs for ct::STANDARD_NO_PAD\n\
         with a dudect-style Welch t-test. This is empirical evidence only."
    );
}
