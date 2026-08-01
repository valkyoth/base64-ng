use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

const INPUT_LEN: usize = 64;
const OUTPUT_LEN: usize = 48;
const ENCODE_OUTPUT_LEN: usize = 88;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Clone, Copy)]
enum TimingCase {
    ValidContents,
    MalformedPosition,
    MalformedClass,
    PreGateContents,
    EncodeBuiltinContents,
    EncodeCustomContents,
}

impl TimingCase {
    const ALL: [Self; 6] = [
        Self::ValidContents,
        Self::MalformedPosition,
        Self::MalformedClass,
        Self::PreGateContents,
        Self::EncodeBuiltinContents,
        Self::EncodeCustomContents,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::ValidContents => "valid-contents-whole-call",
            Self::MalformedPosition => "malformed-position-whole-call",
            Self::MalformedClass => "malformed-class-whole-call",
            Self::PreGateContents => "valid-contents-pre-gate",
            Self::EncodeBuiltinContents => "encode-builtin-input-contents",
            Self::EncodeCustomContents => "encode-custom-input-contents",
        }
    }

    const fn stops_before_gate(self) -> bool {
        matches!(self, Self::PreGateContents)
    }

    const fn encodes(self) -> bool {
        matches!(self, Self::EncodeBuiltinContents | Self::EncodeCustomContents)
    }
}

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
    for case in TimingCase::ALL {
        run_case(config, case, &mut rng)?;
    }
    Ok(())
}

fn run_case(config: Config, case: TimingCase, rng: &mut XorShift64) -> Result<(), String> {
    let mut left = [b'A'; INPUT_LEN];
    let mut right = [b'A'; INPUT_LEN];
    prepare_classes(case, &mut left, &mut right, rng);

    let warmup_first = usize::from((rng.next() & 1) != 0);
    for index in 0..config.warmup {
        refresh_random_class(case, &mut right, rng);
        let class = (index & 1) ^ warmup_first;
        let input = if class == 0 { &left } else { &right };
        measure(input, config.iterations, case)?;
    }

    let mut fixed_stats = Accumulator::new();
    let mut random_stats = Accumulator::new();

    let sample_first = usize::from((rng.next() & 1) != 0);
    for index in 0..config.samples {
        refresh_random_class(case, &mut right, rng);
        let class = (index & 1) ^ sample_first;
        let input = if class == 0 { &left } else { &right };
        let elapsed = measure(input, config.iterations, case)?;

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
        "dudect: case={} samples={} iterations={} fixed_n={} random_n={} fixed_mean_ns={:.3} random_mean_ns={:.3} t={:.3} threshold={:.3}",
        case.label(),
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
            "{} absolute Welch t-statistic {:.3} exceeded threshold {:.3}",
            case.label(),
            t.abs(),
            config.threshold
        ))
    } else {
        Ok(())
    }
}

fn measure(
    input: &[u8; INPUT_LEN],
    iterations: usize,
    case: TimingCase,
) -> Result<f64, String> {
    if case.encodes() {
        measure_encode(input, iterations, case)
    } else {
        measure_decode(input, iterations, case.stops_before_gate())
    }
}

fn measure_decode(
    input: &[u8; INPUT_LEN],
    iterations: usize,
    stop_before_gate: bool,
) -> Result<f64, String> {
    let start = Instant::now();
    for _ in 0..iterations {
        let mut frame = base64_ng::secret::SecretArrayFrame::<OUTPUT_LEN>::new(
            &base64_ng::STRICT_STANDARD_UNPADDED,
        )
        .map_err(|error| format!("secret frame construction failed: {error}"))?;
        frame
            .update(&base64_ng::secret::SecretInput::new(black_box(input)))
            .map_err(|error| format!("secret frame update failed: {error}"))?;
        if stop_before_gate {
            black_box(frame.state().input_len());
        } else {
            black_box(frame.finish().is_ok());
        }
    }
    let nanos = start.elapsed().as_nanos() as f64;
    Ok(nanos / iterations as f64)
}

fn measure_encode(
    input: &[u8; INPUT_LEN],
    iterations: usize,
    case: TimingCase,
) -> Result<f64, String> {
    let start = Instant::now();
    for _ in 0..iterations {
        let encoded = match case {
            TimingCase::EncodeBuiltinContents => {
                base64_ng::secret::SecretArrayEncoder::<ENCODE_OUTPUT_LEN>::encode(
                    &base64_ng::STRICT_STANDARD_PADDED,
                    &base64_ng::secret::SecretInput::new(black_box(input)),
                )
            }
            TimingCase::EncodeCustomContents => {
                base64_ng::secret::SecretArrayEncoder::<ENCODE_OUTPUT_LEN>::encode(
                    &base64_ng::CRYPT_ALPHABET_NO_PAD,
                    &base64_ng::secret::SecretInput::new(black_box(input)),
                )
            }
            _ => return Err("decode timing case reached encode measurement".to_owned()),
        }
        .map_err(|error| format!("secret encoder failed: {error}"))?;
        black_box(encoded.len());
    }
    let nanos = start.elapsed().as_nanos() as f64;
    Ok(nanos / iterations as f64)
}

fn prepare_classes(
    case: TimingCase,
    left: &mut [u8; INPUT_LEN],
    right: &mut [u8; INPUT_LEN],
    rng: &mut XorShift64,
) {
    match case {
        TimingCase::ValidContents | TimingCase::PreGateContents => {
            fill_random_base64(right, rng);
        }
        TimingCase::MalformedPosition => {
            left[0] = b'!';
            right[INPUT_LEN - 1] = b'!';
        }
        TimingCase::MalformedClass => {
            left[INPUT_LEN / 2] = b'!';
            right[INPUT_LEN / 2] = b'=';
        }
        TimingCase::EncodeBuiltinContents | TimingCase::EncodeCustomContents => {
            left.fill(0);
            fill_random_bytes(right, rng);
        }
    }
}

fn refresh_random_class(
    case: TimingCase,
    right: &mut [u8; INPUT_LEN],
    rng: &mut XorShift64,
) {
    match case {
        TimingCase::ValidContents | TimingCase::PreGateContents => {
            fill_random_base64(right, rng);
        }
        TimingCase::EncodeBuiltinContents | TimingCase::EncodeCustomContents => {
            fill_random_bytes(right, rng);
        }
        TimingCase::MalformedPosition | TimingCase::MalformedClass => {}
    }
}

fn fill_random_bytes(output: &mut [u8; INPUT_LEN], rng: &mut XorShift64) {
    for byte in output {
        *byte = rng.next() as u8;
    }
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
         Measures bounded 2.0 secret decode frames across valid contents,\n\
         malformed positions, malformed classes, and the pre-gate core, plus\n\
         built-in and custom-alphabet secret encoding input classes. This is\n\
         empirical evidence only."
    );
}
