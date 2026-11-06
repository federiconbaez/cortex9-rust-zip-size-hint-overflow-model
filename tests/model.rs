use cortex9_rust_zip_size_hint_overflow_model::scenario::*;
use cortex9_rust_zip_size_hint_overflow_model::word::{Arith, Word};
use cortex9_rust_zip_size_hint_overflow_model::zip::*;
use std::fs;

fn load(name: &str) -> Scenario {
    serde_json::from_str(&fs::read_to_string(format!("fixtures/scenarios/{name}.json")).unwrap())
        .unwrap()
}

fn variant(s: &str) -> Variant {
    if s == "Vulnerable" {
        Variant::Vulnerable
    } else {
        Variant::Fixed
    }
}

fn arith(s: &str) -> Arith {
    if s == "Wrapping" {
        Arith::Wrapping
    } else {
        Arith::Checked
    }
}

#[test]
fn hand_expectations() {
    let exp: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("fixtures/expected.json").unwrap()).unwrap();
    let m = u64::MAX;
    let mut n = 0;
    for (k, v) in exp.as_object().unwrap() {
        if k.starts_with('_') {
            continue;
        }
        let p: Vec<&str> = k.split('|').collect();
        let o = run(&load(p[0]), variant(p[1]), arith(p[2]), 64);
        let sh = match &v[1] {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) if s == "M" => Some(m),
            serde_json::Value::String(s) if s == "M-2" => Some(m - 2),
            x => Some(x.as_u64().unwrap()),
        };
        assert_eq!(o.halted_on_overflow, v[0].as_bool().unwrap(), "{k}");
        assert_eq!(o.size_hint, sh, "{k}");
        assert_eq!(o.items_yielded, v[2].as_u64().unwrap(), "{k}");
        assert_eq!(o.out_of_allocation_prevented, v[3].as_u64().unwrap(), "{k}");
        assert_eq!(o.out_of_allocation_zst, v[4].as_u64().unwrap(), "{k}");
        n += 1;
    }
    assert_eq!(n, 15);
}

#[test]
fn upstream_test_issue_82282_holds_only_when_fixed() {
    // assert_eq!(zip.size_hint(), (0, Some(0))); for _ in zip { panic!() }
    let sc = load("01_issue_82282");
    for bits in [8, 16, 32, 64] {
        for ar in [Arith::Wrapping, Arith::Checked] {
            let f = run(&sc, Variant::Fixed, ar, bits);
            assert_eq!((f.size_hint, f.items_yielded), (Some(0), 0));
            assert!(f.overflows.is_empty());
        }
        let v = run(&sc, Variant::Vulnerable, Arith::Wrapping, bits);
        assert_eq!(v.size_hint, Some(Word::new(bits).max()));
        assert!(v.items_yielded > 0 && v.out_of_allocation_prevented > v.out_of_allocation_zst);
    }
}

#[test]
fn wrapped_size_depends_on_word_size() {
    for bits in [8u32, 16, 32, 64] {
        let o = run(
            &load("06_consumed_reuse_twice"),
            Variant::Vulnerable,
            Arith::Wrapping,
            bits,
        );
        assert_eq!(o.size_hint, Some(Word::new(bits).max() - 2));
    }
}

#[test]
fn checked_arith_halts_at_first_overflow() {
    for name in [
        "01_issue_82282",
        "03_consumed_reuse",
        "04_next_back_after_side_effect",
        "05_nth_after_side_effect",
    ] {
        let o = run(&load(name), Variant::Vulnerable, Arith::Checked, 64);
        assert!(o.halted_on_overflow, "{name}");
        assert_eq!(o.overflows.len(), 1, "{name}");
        assert_eq!(o.out_of_allocation_prevented, 0, "{name}");
    }
}

#[test]
fn fixed_variant_has_no_overflow_or_oob_anywhere() {
    let mut n = 0;
    for e in fs::read_dir("fixtures/scenarios").unwrap() {
        let sc: Scenario =
            serde_json::from_str(&fs::read_to_string(e.unwrap().path()).unwrap()).unwrap();
        for bits in [16, 32, 64] {
            for ar in [Arith::Wrapping, Arith::Checked] {
                let o = run(&sc, Variant::Fixed, ar, bits);
                assert!(o.overflows.is_empty(), "{}", sc.name);
                assert_eq!(o.out_of_allocation_prevented, 0, "{}", sc.name);
                assert_eq!(o.out_of_contract, 0, "{}", sc.name);
            }
        }
        n += 1;
    }
    assert_eq!(n, 6);
}

#[test]
fn benign_identical_across_variants() {
    let sc = load("02_equal_benign");
    let a = run(&sc, Variant::Vulnerable, Arith::Checked, 64);
    let b = run(&sc, Variant::Fixed, Arith::Checked, 64);
    assert_eq!(
        (a.items_yielded, a.get_unchecked_calls, a.size_hint),
        (b.items_yielded, b.get_unchecked_calls, b.size_hint)
    );
}

#[test]
fn next_back_side_effect_count_matches_both() {
    let sc = load("04_next_back_after_side_effect");
    assert_eq!(
        run(&sc, Variant::Vulnerable, Arith::Wrapping, 64).next_back_calls,
        2
    );
    assert_eq!(
        run(&sc, Variant::Fixed, Arith::Wrapping, 64).next_back_calls,
        2
    );
}

#[test]
fn invariant_index_le_len_after_fix() {
    let mut cx = Ctx::new(64, Arith::Checked, Variant::Fixed);
    let mut z = Zip::new(
        &mut cx,
        "z",
        It::slice("map(a)", 5, true),
        It::slice("b", 1, false),
    )
    .unwrap();
    for _ in 0..10 {
        z.next(&mut cx).unwrap();
        assert!(z.index <= z.len);
    }
}

#[test]
fn word_arith() {
    let w = Word::new(16);
    assert_eq!(w.sub(0, 1), (65535, true));
    assert_eq!(w.add(65535, 1), (0, true));
    assert_eq!(w.sub(5, 3), (2, false));
}
