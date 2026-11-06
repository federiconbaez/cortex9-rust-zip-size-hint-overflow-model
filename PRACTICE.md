# Guía práctica / Practice guide

Ejercicios **offline y locales** en Rust seguro. No se usa `unsafe`, ni
toolchains viejos, ni se lee memoria fuera de límites. *English summary at
the end.*

## Objetivos

1. Leer la especialización de `Zip` y localizar el invariante roto
   (`index <= len`).
2. Ver cómo un desborde aritmético en `size_hint` se convierte en accesos
   fuera de límites en un `Zip` anidado.
3. Comparar aritmética con wrap y con detección, y el efecto del tamaño de
   palabra.
4. Escribir tus propios invariantes y tests para iteradores con acceso
   aleatorio.

## Prerrequisitos

Rust estable. Lectura: zip.rs en 0148b97, diff del PR #82289, GHSA
(ver [SOURCES.md](SOURCES.md)).

## Comandos locales

```sh
cargo test
cargo run --release --bin zip-model -- run fixtures/scenarios/01_issue_82282.json
cargo run --release --bin zip-model -- matrix fixtures/scenarios /tmp/out && diff -r results /tmp/out
```

## Ejercicios graduados

**Nivel 1 — a mano.** Para el zip interno de 01, escribe `index` y `len`
tras `next()` en ambas variantes y el valor de `size_hint` con W = 16.

**Nivel 2 — anidar.** Explica por qué el `Zip` externo de 01 obtiene
`len = 2^W − 1` y deriva a mano las 22 llamadas y las 16 fuera de la
asignación (10 en el ZST) de los 5 pasos.

**Nivel 3 — wrap vs check.** Compara las filas `Wrapping` y `Checked` de 01,
03 y 05. ¿Qué cambia y qué no?

**Nivel 4 — `nth` y `next_back`.** En 05, ¿por qué `nth` desborda dos veces
en la variante vulnerable? En 04, ¿por qué ambas variantes hacen 2
`next_back` sobre `a`? Relaciónalo con `cmp::max(len, index)`.

**Nivel 5 — invariante.** Escribe un test que verifique `index <= len`
tras cualquier secuencia de `next`/`nth`/`next_back` en la variante
corregida (amplía `invariant_index_le_len_after_fix`).

**Nivel 6 — código.** Añade un escenario nuevo (por ejemplo un `Zip` triple
o `b` con efectos laterales), con expectativas a mano en
`fixtures/expected.json` y tests.

**Nivel 7 — defensa propia.** Para un iterador propio con acceso aleatorio,
escribe un `debug_assert!` de invariantes y un test de propiedades que
recorra secuencias cortas de operaciones.

## Vínculo modelo ↔ CVE

| Caso real | En el código |
|---|---|
| `index += 1` sin `len += 1` | `Zip::next`, rama de efectos laterales, `Variant::Vulnerable` |
| `size_hint` desbordado | `It::size` → evento `Overflow` |
| Acceso fuera de límites | evento `GetUnchecked` con `in_allocation = false` (no ejecutado) |
| Fix PR #82289 y su test | `Variant::Fixed`, escenario 01 |

---

## English summary

Offline exercises in safe Rust only. Levels: (1) derive `index`, `len` and
`size_hint` by hand; (2) derive the nested-zip call counts; (3) compare
wrapping and checked arithmetic; (4) the `nth` and `next_back` paths;
(5) write an `index <= len` invariant test; (6) add a scenario with
hand-derived expectations; (7) apply invariant assertions and property tests
to your own random-access iterator.
