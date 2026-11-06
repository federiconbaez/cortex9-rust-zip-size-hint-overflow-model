#![forbid(unsafe_code)]
use cortex9_rust_zip_size_hint_overflow_model::scenario::*;
use cortex9_rust_zip_size_hint_overflow_model::word::Arith;
use cortex9_rust_zip_size_hint_overflow_model::zip::Variant;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const VARIANTS: [Variant; 2] = [Variant::Vulnerable, Variant::Fixed];
const ARITHS: [Arith; 2] = [Arith::Wrapping, Arith::Checked];
const BITS: [u32; 3] = [16, 32, 64];

fn load(p: &Path) -> Scenario {
    serde_json::from_str(&fs::read_to_string(p).unwrap())
        .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("run") if a.len() == 3 => {
            let sc = load(Path::new(&a[2]));
            let all: Vec<Outcome> = VARIANTS
                .iter()
                .flat_map(|v| ARITHS.iter().map(move |ar| (*v, *ar)))
                .map(|(v, ar)| run(&sc, v, ar, 64))
                .collect();
            println!("{}", serde_json::to_string_pretty(&all).unwrap());
        }
        Some("matrix") if a.len() == 4 => matrix(Path::new(&a[2]), Path::new(&a[3])),
        _ => {
            eprintln!("usage: zip-model run <scenario.json> | matrix <scenarios_dir> <out_dir>");
            std::process::exit(2)
        }
    }
}

fn matrix(dir: &Path, out: &Path) {
    fs::create_dir_all(out).unwrap();
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut csv = String::from(
        "scenario,variant,arith,bits,halted_on_overflow,size_hint,items_yielded,stopped_at_step_cap,overflows,get_unchecked_calls,out_of_contract,out_of_allocation_prevented,out_of_allocation_zst,next_back_calls\n",
    );
    let mut all = Vec::new();
    for f in &files {
        let sc = load(f);
        for v in VARIANTS {
            for ar in ARITHS {
                for b in BITS {
                    let o = run(&sc, v, ar, b);
                    csv.push_str(&format!(
                        "{},{:?},{:?},{},{},{},{},{},{},{},{},{},{},{}\n",
                        o.scenario,
                        o.variant,
                        o.arith,
                        o.bits,
                        o.halted_on_overflow,
                        o.size_hint.map(|x| x.to_string()).unwrap_or_default(),
                        o.items_yielded,
                        o.stopped_at_step_cap,
                        o.overflows.len(),
                        o.get_unchecked_calls,
                        o.out_of_contract,
                        o.out_of_allocation_prevented,
                        o.out_of_allocation_zst,
                        o.next_back_calls
                    ));
                    all.push(o);
                }
            }
        }
    }
    fs::write(out.join("matrix.csv"), &csv).unwrap();
    fs::write(
        out.join("matrix.json"),
        serde_json::to_string_pretty(&all).unwrap() + "\n",
    )
    .unwrap();
    let mut sums = String::new();
    for n in ["matrix.csv", "matrix.json"] {
        let d = Sha256::digest(fs::read(out.join(n)).unwrap());
        sums.push_str(&format!(
            "{}  {n}\n",
            d.iter().map(|x| format!("{x:02x}")).collect::<String>()
        ));
    }
    fs::write(out.join("SHA256SUMS"), sums).unwrap();
    eprintln!("wrote {} rows to {}", all.len(), out.display());
}
