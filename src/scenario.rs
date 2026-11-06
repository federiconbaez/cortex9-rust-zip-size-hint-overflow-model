use crate::word::Arith;
use crate::zip::{Ctx, Event, Halt, It, Variant, Zip};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    /// rust-lang/rust `test_issue_82282`: two copies of
    /// `arr.iter().map(..).zip([(); 0].iter())` advanced once, zipped together.
    Issue82282 { arr_len: u64, max_steps: u64 },
    /// Benign: zip two equal-length side-effect-free slices and drain it.
    Equal { len: u64 },
    /// Drain `map(a).zip(b)` with a > b, call `next` `extra` more times, then
    /// read `size_hint` ("a consumed Zip iterator is used again").
    ConsumedReuse { a_len: u64, b_len: u64, extra: u64 },
    /// `map(a).zip(empty)`: `next`, then `next_back`.
    NextBackAfterSideEffect { a_len: u64 },
    /// `map(a).zip(empty)`: `next`, then `nth(n)`.
    NthAfterSideEffect { a_len: u64, n: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub kind: Kind,
}

/// Slices whose name starts with this prefix model `[(); 0].iter()`
/// (zero-sized elements: an out-of-allocation index reads no bytes).
pub const ZST_PREFIX: &str = "empty";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// The scenario ran to its observation point.
    Completed,
    /// `Arith::Checked` stopped at the first overflow (a panic in real code).
    HaltedOnOverflow,
    /// A loop exceeded the model's cap (`LOOP_CAP` or the scenario's
    /// `max_steps`). Counters cover only what ran before the stop
    /// (`counts_partial`); what real code does afterwards is indeterminate.
    ModelLimit,
    /// A scenario parameter does not fit in the chosen word size; nothing ran.
    NotRepresentable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelLimitInfo {
    pub site: String,
    /// Exact iteration count real code would run (decimal string).
    pub requested: String,
    pub cap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Outcome {
    pub scenario: String,
    pub variant: Variant,
    pub arith: Arith,
    pub bits: u32,
    pub status: Status,
    pub model_limit: Option<ModelLimitInfo>,
    /// True when counters below stop short of what real code would do.
    pub counts_partial: bool,
    /// size_hint().0 of the iterator under study at the observation point,
    /// or `None` if the run stopped before reaching it.
    pub size_hint: Option<u64>,
    pub items_yielded: u64,
    pub overflows: Vec<String>,
    pub get_unchecked_calls: u64,
    pub out_of_contract: u64,
    /// Calls that would have indexed outside the backing allocation in real
    /// code; here they are only counted.
    pub out_of_allocation_prevented: u64,
    /// Subset of the above on zero-sized-element slices.
    pub out_of_allocation_zst: u64,
    pub next_back_calls: u64,
}

fn inner(cx: &mut Ctx, tag: &str, arr_len: u64) -> Result<It, Halt> {
    let a = It::slice(&format!("map(arr{tag})"), arr_len, true);
    let b = It::slice(&format!("{ZST_PREFIX}{tag}"), 0, false);
    Ok(It::Zip(Zip::new(cx, &format!("zip{tag}"), a, b)?))
}

fn body(cx: &mut Ctx, k: &Kind, out: &mut Outcome) -> Result<(), Halt> {
    match *k {
        Kind::Issue82282 { arr_len, max_steps } => {
            let mut z1 = inner(cx, "1", arr_len)?;
            z1.next(cx)?;
            let mut z2 = inner(cx, "2", arr_len)?;
            z2.next(cx)?;
            let mut outer = It::Zip(Zip::new(cx, "outer", z1, z2)?);
            let sh = outer.size(cx)?;
            out.size_hint = Some(sh);
            // `for _ in zip` in the upstream test: real code would call next()
            // until it returns None. The scenario simulates at most max_steps.
            while out.items_yielded < max_steps {
                if !outer.next(cx)? {
                    return Ok(());
                }
                out.items_yielded += 1;
            }
            cx.events.push(Event::ModelLimit {
                site: "scenario: for _ in zip (max_steps)".into(),
                requested: format!("at least {}", max_steps + 1),
                cap: max_steps,
            });
            return Err(Halt::ModelLimit);
        }
        Kind::Equal { len } => {
            let mut z = It::Zip(Zip::new(
                cx,
                "zip",
                It::slice("a", len, false),
                It::slice("b", len, false),
            )?);
            cx.bounded("scenario: drain Equal", len as u128 + 1)?;
            while z.next(cx)? {
                out.items_yielded += 1;
            }
            out.size_hint = Some(z.size(cx)?);
        }
        Kind::ConsumedReuse {
            a_len,
            b_len,
            extra,
        } => {
            let mut z = It::Zip(Zip::new(
                cx,
                "zip",
                It::slice("map(a)", a_len, true),
                It::slice("b", b_len, false),
            )?);
            cx.bounded(
                "scenario: drain ConsumedReuse",
                a_len.min(b_len) as u128 + 1,
            )?;
            while z.next(cx)? {
                out.items_yielded += 1;
            }
            let extra = cx.bounded("scenario: extra next() calls", extra as u128)?;
            for _ in 0..extra {
                z.next(cx)?;
            }
            out.size_hint = Some(z.size(cx)?);
        }
        Kind::NextBackAfterSideEffect { a_len } => {
            let mut z = inner(cx, "", a_len)?;
            z.next(cx)?;
            if z.next_back(cx)? {
                out.items_yielded += 1;
            }
            out.size_hint = Some(z.size(cx)?);
        }
        Kind::NthAfterSideEffect { a_len, n } => {
            let mut z = inner(cx, "", a_len)?;
            z.next(cx)?;
            if let It::Zip(zz) = &mut z
                && zz.nth(cx, n)?
            {
                out.items_yielded += 1;
            }
            out.size_hint = Some(z.size(cx)?);
        }
    }
    Ok(())
}

impl Kind {
    /// Parameters that become model words (lengths, `n`). `max_steps` is a
    /// harness bound, not a word.
    fn words(&self) -> Vec<u64> {
        match *self {
            Kind::Issue82282 { arr_len, .. } => vec![arr_len],
            Kind::Equal { len } => vec![len],
            Kind::ConsumedReuse { a_len, b_len, .. } => vec![a_len, b_len],
            Kind::NextBackAfterSideEffect { a_len } => vec![a_len],
            Kind::NthAfterSideEffect { a_len, n } => vec![a_len, n],
        }
    }
}

pub fn run(sc: &Scenario, variant: Variant, arith: Arith, bits: u32) -> Outcome {
    let mut cx = Ctx::new(bits, arith, variant);
    let mut out = Outcome {
        scenario: sc.name.clone(),
        variant,
        arith,
        bits,
        status: Status::Completed,
        model_limit: None,
        counts_partial: false,
        size_hint: None,
        items_yielded: 0,
        overflows: vec![],
        get_unchecked_calls: 0,
        out_of_contract: 0,
        out_of_allocation_prevented: 0,
        out_of_allocation_zst: 0,
        next_back_calls: 0,
    };
    if sc.kind.words().iter().any(|&w| w > cx.word.max()) {
        out.status = Status::NotRepresentable;
        return out;
    }
    match body(&mut cx, &sc.kind, &mut out) {
        Ok(()) => {}
        Err(Halt::Overflow) => out.status = Status::HaltedOnOverflow,
        Err(Halt::ModelLimit) => {
            out.status = Status::ModelLimit;
            out.counts_partial = true;
        }
    }
    for e in &cx.events {
        match e {
            Event::Overflow { site, .. } => out.overflows.push(site.clone()),
            Event::GetUnchecked {
                iter,
                in_contract,
                in_allocation,
                ..
            } => {
                out.get_unchecked_calls += 1;
                out.out_of_contract += u64::from(!in_contract);
                out.out_of_allocation_prevented += u64::from(!in_allocation);
                out.out_of_allocation_zst +=
                    u64::from(!in_allocation && iter.starts_with(ZST_PREFIX));
            }
            Event::NextBack { .. } => out.next_back_calls += 1,
            Event::ModelLimit {
                site,
                requested,
                cap,
            } => {
                out.model_limit = Some(ModelLimitInfo {
                    site: site.clone(),
                    requested: requested.clone(),
                    cap: *cap,
                })
            }
        }
    }
    out
}
