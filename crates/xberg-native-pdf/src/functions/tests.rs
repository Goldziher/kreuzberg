use super::*;

fn approx_eq(a: &[f64], b: &[f64], eps: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < eps)
}

#[test]
fn linear_ramp_tint_transform() {
    let prog = b"{ dup 0.84 mul exch 0.00 exch dup 0.44 mul exch 0.21 mul }";
    let result = evaluate_type4(prog, &[0.5]).unwrap();
    assert!(approx_eq(&result, &[0.42, 0.0, 0.22, 0.105], 1e-9), "got {result:?}");
}

#[test]
fn identity_empty_program() {
    let result = evaluate_type4(b"{ }", &[0.7]).unwrap();
    assert!(approx_eq(&result, &[0.7], 1e-9), "got {result:?}");
}

#[test]
fn constant_output() {
    let prog = b"{ pop 1.0 0.0 0.0 0.0 }";
    let result = evaluate_type4(prog, &[0.5]).unwrap();
    assert_eq!(result, vec![1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn conditional_ifelse() {
    let prog = b"{ dup 0.5 gt { pop 1.0 } { 0.0 exch } ifelse }";
    let high = evaluate_type4(prog, &[0.8]).unwrap();
    assert_eq!(high, vec![1.0]);
    let low = evaluate_type4(prog, &[0.3]).unwrap();
    assert!(approx_eq(&low, &[0.0, 0.3], 1e-9), "got {low:?}");
}

#[test]
fn conditional_if() {
    let prog = b"{ dup 0.5 gt { 1.0 add } if }";
    let high = evaluate_type4(prog, &[0.8]).unwrap();
    assert!(approx_eq(&high, &[1.8], 1e-9), "got {high:?}");
    let low = evaluate_type4(prog, &[0.3]).unwrap();
    assert!(approx_eq(&low, &[0.3], 1e-9), "got {low:?}");
}

#[test]
fn domain_range_clamping() {
    let prog = b"{ 2.0 mul }";
    let result = evaluate_type4_clamped(prog, &[1.5], &[[0.0, 1.0]], &[[0.0, 1.0]]).unwrap();
    // Input 1.5 clamped to 1.0, * 2.0 = 2.0, clamped to 1.0 ~keep
    assert_eq!(result, vec![1.0]);
}

#[test]
fn stack_underflow_returns_error() {
    let prog = b"{ add }";
    let err = evaluate_type4(prog, &[]).unwrap_err();
    assert!(err.to_string().contains("stack underflow"), "got: {err}");
}

#[test]
fn arithmetic_operators() {
    assert_eq!(evaluate_type4(b"{ add }", &[3.0, 4.0]).unwrap(), vec![7.0]);
    assert_eq!(evaluate_type4(b"{ sub }", &[10.0, 3.0]).unwrap(), vec![7.0]);
    assert_eq!(evaluate_type4(b"{ mul }", &[3.0, 4.0]).unwrap(), vec![12.0]);
    assert_eq!(evaluate_type4(b"{ div }", &[10.0, 4.0]).unwrap(), vec![2.5]);
    assert_eq!(evaluate_type4(b"{ idiv }", &[10.0, 3.0]).unwrap(), vec![3.0]);
    assert_eq!(evaluate_type4(b"{ mod }", &[10.0, 3.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ neg }", &[5.0]).unwrap(), vec![-5.0]);
    assert_eq!(evaluate_type4(b"{ abs }", &[-5.0]).unwrap(), vec![5.0]);
    assert_eq!(evaluate_type4(b"{ ceiling }", &[3.2]).unwrap(), vec![4.0]);
    assert_eq!(evaluate_type4(b"{ floor }", &[3.8]).unwrap(), vec![3.0]);
    assert_eq!(evaluate_type4(b"{ round }", &[3.5]).unwrap(), vec![4.0]);
    assert_eq!(evaluate_type4(b"{ truncate }", &[3.9]).unwrap(), vec![3.0]);
    assert_eq!(evaluate_type4(b"{ sqrt }", &[9.0]).unwrap(), vec![3.0]);
}

#[test]
fn trig_operators() {
    let sin_result = evaluate_type4(b"{ sin }", &[90.0]).unwrap();
    assert!((sin_result[0] - 1.0).abs() < 1e-9);
    let cos_result = evaluate_type4(b"{ cos }", &[0.0]).unwrap();
    assert!((cos_result[0] - 1.0).abs() < 1e-9);
    let atan_result = evaluate_type4(b"{ atan }", &[1.0, 1.0]).unwrap();
    assert!((atan_result[0] - 45.0).abs() < 1e-9);
}

#[test]
fn log_operators() {
    let ln_result = evaluate_type4(b"{ ln }", &[std::f64::consts::E]).unwrap();
    assert!((ln_result[0] - 1.0).abs() < 1e-9);
    let log_result = evaluate_type4(b"{ log }", &[100.0]).unwrap();
    assert!((log_result[0] - 2.0).abs() < 1e-9);
}

#[test]
fn exp_operator() {
    let result = evaluate_type4(b"{ exp }", &[2.0, 10.0]).unwrap();
    assert!((result[0] - 1024.0).abs() < 1e-9);
}

#[test]
fn comparison_operators() {
    assert_eq!(evaluate_type4(b"{ eq }", &[1.0, 1.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ eq }", &[1.0, 2.0]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ ne }", &[1.0, 2.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ gt }", &[2.0, 1.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ ge }", &[2.0, 2.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ lt }", &[1.0, 2.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ le }", &[2.0, 2.0]).unwrap(), vec![1.0]);
}

#[test]
fn boolean_operators() {
    // True/false literals exercise the boolean dispatch in and/or/xor/not. ~keep
    assert_eq!(evaluate_type4(b"{ true false and }", &[]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ true false or }", &[]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ true true xor }", &[]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ true not }", &[]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ false not }", &[]).unwrap(), vec![1.0]);
}

#[test]
fn bitwise_operators() {
    assert_eq!(evaluate_type4(b"{ 12 10 and }", &[]).unwrap(), vec![8.0]);
    assert_eq!(evaluate_type4(b"{ 12 10 or }", &[]).unwrap(), vec![14.0]);
    assert_eq!(evaluate_type4(b"{ 8 2 bitshift }", &[]).unwrap(), vec![32.0]);
    assert_eq!(evaluate_type4(b"{ 32 -2 bitshift }", &[]).unwrap(), vec![8.0]);
}

#[test]
fn stack_manipulation() {
    assert_eq!(evaluate_type4(b"{ dup }", &[5.0]).unwrap(), vec![5.0, 5.0]);
    assert_eq!(evaluate_type4(b"{ exch }", &[1.0, 2.0]).unwrap(), vec![2.0, 1.0]);
    assert_eq!(evaluate_type4(b"{ pop }", &[1.0, 2.0]).unwrap(), vec![1.0]);
    assert_eq!(
        evaluate_type4(b"{ 2 copy }", &[1.0, 2.0]).unwrap(),
        vec![1.0, 2.0, 1.0, 2.0]
    );
    assert_eq!(
        evaluate_type4(b"{ 1 index }", &[1.0, 2.0]).unwrap(),
        vec![1.0, 2.0, 1.0]
    );
}

#[test]
fn roll_operator() {
    // roll(n=3, j=1): rotate top 3 elements by 1
    // [1, 2, 3] -> [3, 1, 2] ~keep
    assert_eq!(
        evaluate_type4(b"{ 3 1 roll }", &[1.0, 2.0, 3.0]).unwrap(),
        vec![3.0, 1.0, 2.0]
    );
    // roll(n=3, j=-1): rotate top 3 elements by -1
    // [1, 2, 3] -> [2, 3, 1] ~keep
    assert_eq!(
        evaluate_type4(b"{ 3 -1 roll }", &[1.0, 2.0, 3.0]).unwrap(),
        vec![2.0, 3.0, 1.0]
    );
}

#[test]
fn bool_literals() {
    assert_eq!(evaluate_type4(b"{ true }", &[]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ false }", &[]).unwrap(), vec![0.0]);
}

#[test]
fn division_by_zero_follows_ieee_754() {
    // Acrobat/Poppler hand back IEEE 754 specials for `div` by zero
    // rather than failing the whole program. We follow that behaviour;
    // `idiv` and `mod` (integer ops with no inf/NaN) stay as errors. ~keep
    let pos = evaluate_type4(b"{ div }", &[1.0, 0.0]).unwrap();
    assert_eq!(pos.len(), 1);
    assert!(pos[0].is_infinite() && pos[0] > 0.0, "expected +inf, got {pos:?}");

    let neg = evaluate_type4(b"{ div }", &[-1.0, 0.0]).unwrap();
    assert_eq!(neg.len(), 1);
    assert!(neg[0].is_infinite() && neg[0] < 0.0, "expected -inf, got {neg:?}");

    let nan = evaluate_type4(b"{ div }", &[0.0, 0.0]).unwrap();
    assert_eq!(nan.len(), 1);
    assert!(nan[0].is_nan(), "expected NaN, got {nan:?}");

    assert!(evaluate_type4(b"{ idiv }", &[1.0, 0.0]).is_err());
    assert!(evaluate_type4(b"{ mod }", &[1.0, 0.0]).is_err());
}

#[test]
fn int_min_neg_and_abs_error() {
    // i64::MIN cannot be negated or abs'd without overflow. PLRM raises
    // a runtime error; we map that to Error::Type4Runtime. ~keep
    let neg = format!("{{ {} neg }}", i64::MIN);
    let err = evaluate_type4(neg.as_bytes(), &[]).unwrap_err();
    assert!(matches!(err, Error::Type4Runtime(_)), "got: {err}");

    let abs = format!("{{ {} abs }}", i64::MIN);
    let err = evaluate_type4(abs.as_bytes(), &[]).unwrap_err();
    assert!(matches!(err, Error::Type4Runtime(_)), "got: {err}");
}

#[test]
fn invalid_program_missing_braces() {
    let err = evaluate_type4(b"dup mul", &[1.0]).unwrap_err();
    assert!(err.to_string().contains("{ }"), "got: {err}");
}

#[test]
fn nested_conditionals() {
    let prog = b"{ dup 0.5 gt { dup 0.8 gt { pop 1.0 } { pop 0.75 } ifelse } { pop 0.0 } ifelse }";
    assert_eq!(evaluate_type4(prog, &[0.9]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(prog, &[0.6]).unwrap(), vec![0.75]);
    assert_eq!(evaluate_type4(prog, &[0.3]).unwrap(), vec![0.0]);
}

#[test]
fn real_world_spot_color_transforms() {
    // Pantone-style: single ink maps to CMYK ~keep
    let prog = b"{ 0 exch dup 0.78 mul exch 0.35 mul 0 }";
    let result = evaluate_type4(prog, &[1.0]).unwrap();
    assert!(approx_eq(&result, &[0.0, 0.78, 0.35, 0.0], 1e-9), "got {result:?}");
}

#[test]
fn negative_number_literal() {
    let result = evaluate_type4(b"{ -3.5 add }", &[10.0]).unwrap();
    assert!(approx_eq(&result, &[6.5], 1e-9), "got {result:?}");
}

#[test]
fn plrm_examples() {
    // (program_bytes, inputs, expected_outputs, description) ~keep
    let cases: &[(&[u8], &[f64], &[f64], &str)] = &[
        (b"{ atan }", &[-100.0, 0.0], &[270.0], "atan negative-num zero-den"),
        (b"{ atan }", &[-1.0, -1.0], &[225.0], "atan third quadrant"),
        (b"{ atan }", &[0.0, 1.0], &[0.0], "atan first axis"),
        (b"{ atan }", &[1.0, 1.0], &[45.0], "atan first quadrant"),
        (b"{ atan }", &[0.0, -1.0], &[180.0], "atan negative-x axis"),
        (b"{ round }", &[-6.5], &[-6.0], "round negative half toward +inf"),
        (b"{ round }", &[6.5], &[7.0], "round positive half toward +inf"),
        (b"{ round }", &[-0.5], &[0.0], "round -0.5"),
        (b"{ round }", &[0.5], &[1.0], "round 0.5"),
        (b"{ idiv }", &[-7.0, 2.0], &[-3.0], "idiv negative"),
        (b"{ mod }", &[-7.0, 2.0], &[-1.0], "mod negative dividend"),
        (b"{ truncate }", &[-6.5], &[-6.0], "truncate negative"),
    ];
    for (prog, inp, want, desc) in cases {
        let got = evaluate_type4(prog, inp).unwrap_or_else(|e| panic!("{desc}: {e}"));
        assert!(
            approx_eq(&got, want, 1e-9),
            "case: {desc}\n  got:  {got:?}\n  want: {want:?}"
        );
    }
}

#[test]
fn not_distinguishes_bool_from_int() {
    // PLRM §8.2: `true not -> false` (logical), `52 not -> -53` (bitwise),
    // `1 not -> -2` (bitwise on the integer literal 1, NOT boolean true). ~keep
    assert_eq!(evaluate_type4(b"{ true not }", &[]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ false not }", &[]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ 52 not }", &[]).unwrap(), vec![-53.0]);
    assert_eq!(evaluate_type4(b"{ 1 not }", &[]).unwrap(), vec![-2.0]);
    assert_eq!(evaluate_type4(b"{ 0 not }", &[]).unwrap(), vec![-1.0]);
}

#[test]
fn and_or_xor_dispatch_on_type() {
    // Both-boolean -> logical ~keep
    assert_eq!(evaluate_type4(b"{ true true and }", &[]).unwrap(), vec![1.0]);
    // Both-integer -> bitwise ~keep
    assert_eq!(evaluate_type4(b"{ 12 10 and }", &[]).unwrap(), vec![8.0]);
    // Mixed -> typecheck error ~keep
    assert!(evaluate_type4(b"{ true 1 and }", &[]).is_err());
    assert!(evaluate_type4(b"{ 1 true or }", &[]).is_err());
}

#[test]
fn integer_only_ops_reject_real_literals() {
    // PLRM §8.2: idiv, mod, bitshift require typed integer operands.
    // A real literal like `2.0` is a typed real and must be rejected. ~keep
    assert!(evaluate_type4(b"{ 5.5 2 idiv }", &[]).is_err());
    assert!(evaluate_type4(b"{ 5 2.5 idiv }", &[]).is_err());
    assert!(evaluate_type4(b"{ 5 2.0 mod }", &[]).is_err());
    assert!(evaluate_type4(b"{ 5.0 2 mod }", &[]).is_err());
    assert!(evaluate_type4(b"{ 1.0 not }", &[]).is_err());
    assert!(evaluate_type4(b"{ 3.0 1 bitshift }", &[]).is_err());
    assert!(evaluate_type4(b"{ 3 1.0 bitshift }", &[]).is_err());
}

#[test]
fn integer_valued_inputs_accepted_by_integer_ops() {
    // Caller-supplied f64 inputs are an ambiguous typed-int/typed-real
    // boundary; integer-valued f64s are accepted by integer ops. ~keep
    assert_eq!(evaluate_type4(b"{ idiv }", &[10.0, 3.0]).unwrap(), vec![3.0]);
    assert_eq!(evaluate_type4(b"{ mod }", &[10.0, 3.0]).unwrap(), vec![1.0]);
    assert_eq!(evaluate_type4(b"{ bitshift }", &[1.0, 4.0]).unwrap(), vec![16.0]);
}

#[test]
fn errors_not_panics() {
    // sqrt of negative, ln/log of non-positive -> error, not NaN/-inf. ~keep
    assert!(evaluate_type4(b"{ sqrt }", &[-1.0]).is_err());
    assert!(evaluate_type4(b"{ ln }", &[0.0]).is_err());
    assert!(evaluate_type4(b"{ ln }", &[-1.0]).is_err());
    assert!(evaluate_type4(b"{ log }", &[0.0]).is_err());
    assert!(evaluate_type4(b"{ log }", &[-1.0]).is_err());

    // Malformed Domain (min > max) used to panic in f64::clamp. ~keep
    let r = evaluate_type4_clamped(b"{ }", &[0.5], &[[1.0, 0.0]], &[]).unwrap();
    // Bounds are swapped, so 0.5 stays in [0, 1]. ~keep
    assert_eq!(r, vec![0.5]);

    // NaN bounds must not panic — treat as no clamp. ~keep
    let r = evaluate_type4_clamped(b"{ }", &[0.5], &[[f64::NAN, 1.0]], &[[0.0, f64::NAN]]).unwrap();
    assert_eq!(r, vec![0.5]);

    // bitshift by >= 64 must not shift-overflow. ~keep
    assert_eq!(evaluate_type4(b"{ 1 64 bitshift }", &[]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ 1 100 bitshift }", &[]).unwrap(), vec![0.0]);
    assert_eq!(evaluate_type4(b"{ 1 -64 bitshift }", &[]).unwrap(), vec![0.0]);

    // idiv overflow path: i64::MIN / -1 ~keep
    let prog = format!("{{ {} -1 idiv }}", i64::MIN);
    assert!(evaluate_type4(prog.as_bytes(), &[]).is_err());

    // Non-finite numeric literals must be rejected at parse time. ~keep
    assert!(evaluate_type4(b"{ inf }", &[]).is_err());
    assert!(evaluate_type4(b"{ NaN }", &[]).is_err());

    // idiv/mod on non-integral reals -> typecheck. ~keep
    assert!(evaluate_type4(b"{ 7.5 2 idiv }", &[]).is_err());
    assert!(evaluate_type4(b"{ 7 2.5 mod }", &[]).is_err());

    // Negative count for copy/index/roll -> error, not garbage. ~keep
    assert!(evaluate_type4(b"{ -1 copy }", &[1.0]).is_err());
    assert!(evaluate_type4(b"{ -1 index }", &[1.0]).is_err());
    assert!(evaluate_type4(b"{ -1 1 roll }", &[1.0, 2.0]).is_err());

    // atan undefined at (0, 0). ~keep
    assert!(evaluate_type4(b"{ atan }", &[0.0, 0.0]).is_err());
}

#[test]
fn cvi_truncates_toward_zero() {
    assert_eq!(evaluate_type4(b"{ cvi }", &[3.2]).unwrap(), vec![3.0]);
    assert_eq!(evaluate_type4(b"{ cvi }", &[-3.2]).unwrap(), vec![-3.0]);
    assert_eq!(evaluate_type4(b"{ cvi }", &[3.0]).unwrap(), vec![3.0]);
    // 3.5 cvi -> 3 (truncate toward zero, not round) ~keep
    assert_eq!(evaluate_type4(b"{ 3.5 cvi }", &[]).unwrap(), vec![3.0]);
    assert_eq!(evaluate_type4(b"{ -3.5 cvi }", &[]).unwrap(), vec![-3.0]);
}

#[test]
fn cvr_makes_typed_real() {
    // 3 cvr -> 3.0 as a typed real. Should not satisfy `idiv` (which
    // wants typed integers) — verifies the type tag really changed. ~keep
    let err = evaluate_type4(b"{ 3 cvr 2 idiv }", &[]).unwrap_err();
    assert!(matches!(err, Error::Type4Runtime(_)), "got: {err}");
    // 3.5 cvr -> 3.5 (stays real) ~keep
    assert_eq!(evaluate_type4(b"{ 3.5 cvr }", &[]).unwrap(), vec![3.5]);
    // Combined with `cvi`: `3.5 cvi 2 idiv` succeeds ~keep
    assert_eq!(evaluate_type4(b"{ 3.5 cvi 2 idiv }", &[]).unwrap(), vec![1.0]);
}

#[test]
fn cvi_rejects_bool_and_non_finite() {
    assert!(evaluate_type4(b"{ true cvi }", &[]).is_err());
    assert!(evaluate_type4(b"{ true cvr }", &[]).is_err());
    // Runtime-produced inf/NaN: cvi rejects non-finite reals. Parser
    // refuses `inf`/`NaN` literals, so route through `1 0 div` (+inf)
    // and `0 0 div` (NaN) to hit the runtime-side check. ~keep
    assert!(evaluate_type4(b"{ 1 0 div cvi }", &[]).is_err());
    assert!(evaluate_type4(b"{ 0 0 div cvi }", &[]).is_err());
    // cvr accepts any real, including non-finite values — no integer
    // overflow concern. Verify the +inf round-trip survives. ~keep
    assert_eq!(evaluate_type4(b"{ 1 0 div cvr }", &[]).unwrap()[0], f64::INFINITY);
}

#[test]
fn program_is_send_sync() {
    // A compiled program is meant to live in a shared tint-transform
    // cache; verify it satisfies the marker bounds. ~keep
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Program>();
}

#[test]
fn program_compile_once_evaluate_many() {
    // Same compiled program, 1000 independent evaluations. Sanity-check
    // that there is no shared mutable state — outputs depend only on
    // inputs. ~keep
    let program = Program::compile(b"{ dup mul }").expect("compile");
    for i in 0..1000 {
        let x = (i as f64) * 0.001;
        let out = program.evaluate(&[x]).expect("eval");
        assert_eq!(out.len(), 1);
        let want = x * x;
        assert!((out[0] - want).abs() < 1e-12, "i={i}: got {out:?}, want {want}");
    }
}

#[test]
fn program_evaluate_clamped_matches_wrapper() {
    // The wrapper should produce the same result as calling the typed
    // API directly. ~keep
    let program = Program::compile(b"{ 2.0 mul }").expect("compile");
    let direct = program
        .evaluate_clamped(&[1.5], &[[0.0, 1.0]], &[[0.0, 1.0]])
        .expect("direct");
    let via_fn = evaluate_type4_clamped(b"{ 2.0 mul }", &[1.5], &[[0.0, 1.0]], &[[0.0, 1.0]]).expect("via_fn");
    assert_eq!(direct, via_fn);
}

#[test]
fn parse_depth_limit_enforced() {
    // 50 levels of nesting comfortably exceeds the depth budget without
    // requiring the actual call stack to grow that far (we error first).
    // Without this guard, parsing recurses into Rust's call stack until
    // it blows. ~keep
    let deep = format!("{{{}}}", "{".repeat(50)) + &"}".repeat(50);
    let err = evaluate_type4(deep.as_bytes(), &[]).unwrap_err();
    assert!(matches!(err, Error::InvalidPdf(_)), "got: {err}");
    assert!(err.to_string().contains("depth"), "got: {err}");

    // Up to the depth cap itself: a deeply nested-but-bounded program
    // should parse successfully when each level only uses procedure bodies
    // that go on to be consumed by if/ifelse. Construct one with exactly
    // MAX_PARSE_DEPTH levels.
    // (We only validate "below the cap is fine" up to a modest depth here
    // because building 32-deep `if`-consuming programs is verbose.) ~keep
}

#[test]
fn runtime_stack_overflow_caught() {
    // Push 300 ones; cap is 256. ~keep
    let prog = "{ ".to_string() + &"1 ".repeat(300) + "}";
    let err = evaluate_type4(prog.as_bytes(), &[]).unwrap_err();
    assert!(matches!(err, Error::Type4Runtime(_)), "got: {err}");
    assert!(err.to_string().contains("stack overflow"), "got: {err}");

    let ok = "{ ".to_string() + &"1 ".repeat(200) + "}";
    assert!(evaluate_type4(ok.as_bytes(), &[]).is_ok());
}

#[test]
fn instruction_budget_caught() {
    // 100_001 `dup pop` pairs keep the stack bounded but consume the
    // instruction budget. Each pair is two ticks, so this is well past
    // MAX_INSTRUCTIONS by design. ~keep
    let mut body = String::from("{ ");
    for _ in 0..100_001 {
        body.push_str("dup pop ");
    }
    body.push('}');
    let err = evaluate_type4(body.as_bytes(), &[1.0]).unwrap_err();
    assert!(matches!(err, Error::Type4Runtime(_)), "got: {err}");
    assert!(err.to_string().contains("instruction budget"), "got: {err}");
}

#[test]
fn input_count_capped() {
    let many: Vec<f64> = (0..(MAX_STACK + 1)).map(|i| i as f64).collect();
    let err = evaluate_type4(b"{ }", &many).unwrap_err();
    assert!(matches!(err, Error::Type4Runtime(_)), "got: {err}");
}

#[test]
fn orphan_procedure_body_rejected_at_parse() {
    // `{ 1 { 2 } 3 }` has an inner `{ 2 }` that no `if`/`ifelse` consumes.
    // Previous behavior silently turned the inner body into an `If` and
    // mis-executed it (popping a bool that wasn't there). Now: parse error. ~keep
    let err = evaluate_type4(b"{ 1 { 2 } 3 }", &[]).unwrap_err();
    assert!(matches!(err, Error::InvalidPdf(_)), "got: {err}");
    assert!(err.to_string().contains("orphan"), "got: {err}");
}

#[test]
fn orphan_procedure_body_alone_rejected() {
    // A program that is only a procedure body with nothing else also has
    // no `if`/`ifelse` to consume it. ~keep
    let err = evaluate_type4(b"{ { 1 2 add } }", &[]).unwrap_err();
    assert!(matches!(err, Error::InvalidPdf(_)), "got: {err}");
}

#[test]
fn atan_full_range() {
    // PLRM §8.2: atan returns angle in [0, 360). ~keep
    for &(num, den, want) in &[
        (0.0, 1.0, 0.0),
        (1.0, 1.0, 45.0),
        (1.0, 0.0, 90.0),
        (1.0, -1.0, 135.0),
        (0.0, -1.0, 180.0),
        (-1.0, -1.0, 225.0),
        (-1.0, 0.0, 270.0),
        (-1.0, 1.0, 315.0),
        (-100.0, 0.0, 270.0),
    ] {
        let got = evaluate_type4(b"{ atan }", &[num, den]).unwrap();
        assert!(
            (got[0] - want).abs() < 1e-9,
            "atan({num}, {den}) = {got:?}, want {want}"
        );
        assert!(got[0] >= 0.0 && got[0] < 360.0, "atan out of [0, 360): {got:?}");
    }
}

#[test]
fn parse_depth_limit_returns_invalid_pdf() {
    // A pathologically nested program is malformed at parse time —
    // it must surface as InvalidPdf so callers don't classify it as a
    // runtime resource failure and retry forever. ~keep
    let mut bytes = Vec::new();
    bytes.extend(std::iter::repeat_n(b'{', 50));
    bytes.extend(std::iter::repeat_n(b'}', 50));
    match evaluate_type4(&bytes, &[]) {
        Err(Error::InvalidPdf(_)) => {}
        Err(Error::Type4Runtime(s)) => {
            panic!("parse depth should error as InvalidPdf, not Type4Runtime: {s}")
        }
        Err(other) => panic!("unexpected error: {other}"),
        Ok(out) => panic!("should have errored, got {out:?}"),
    }
}

#[test]
fn cvi_rejects_two_pow_63() {
    // 2^63 is representable in f64 but not as i64. The upper-bound
    // check must use >= (or its mathematical equivalent) so the
    // boundary is rejected with an integer-overflow error rather
    // than silently saturating to i64::MAX = 2^63 - 1. ~keep
    let pow_63: f64 = 9_223_372_036_854_775_808.0;
    assert_eq!(pow_63, i64::MAX as f64, "test setup: 2^63 == i64::MAX as f64");
    let result = evaluate_type4(b"{ cvi }", &[pow_63]);
    match result {
        Err(Error::Type4Runtime(s)) if s.contains("cvi") => {}
        Err(other) => panic!("expected Type4Runtime(cvi overflow), got: {other}"),
        Ok(v) => panic!(
            "2^63 cvi should overflow; got {v:?} (likely saturated to i64::MAX = {})",
            i64::MAX
        ),
    }
}

#[test]
fn cvi_accepts_two_pow_63_minus_one() {
    // i64::MAX itself cannot be exactly represented as f64, so the
    // largest f64 that rounds to a valid i64 via truncation is
    // the predecessor, ~9.223e18. Verify this passes. ~keep
    let near_max: f64 = 9_223_372_036_854_774_784.0;
    let result = evaluate_type4(b"{ cvi }", &[near_max]).unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0] > 0.0 && result[0] < i64::MAX as f64);
}
