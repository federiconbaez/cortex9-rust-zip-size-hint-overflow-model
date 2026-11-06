# Sources

Local copies under `security-research-45/sources/next/` (not in this repo):

| Source | Used for | SHA-256 of copy |
|---|---|---|
| rust-lang/rust `library/core/src/iter/adapters/zip.rs` at 0148b971c921a0831fbf3357e5936eec724e3566 | `Variant::Vulnerable` (`next`, `nth`, `next_back`, `size_hint`, `new`) and `TrustedRandomAccess::size` | 7efc568bb88f7d8f6c056d68969d69e503f6caecde6efdfba96b7e3083fe3532 |
| rust-lang/rust PR #82289 diff (merged 2021-03-05), issue #82282 | `Variant::Fixed`, `test_issue_82282` | 0bd6b33678a8f56249f5ea800c23747198a88594573436ba8578ab8dc380a8cd |
| GHSA-rq3p-62m7-c66f (CVE-2021-28879) | description, "before 1.52.0", CWE-190 | 91e7d110fd8781510e6cc46d57955c8ab339a88288314bf5ddd25b363c3fa0b9 |

Note: the vulnerable source uses `may_have_side_effect()`; the PR base uses
the constant `MAY_HAVE_SIDE_EFFECT`. The logic modelled is the same.
