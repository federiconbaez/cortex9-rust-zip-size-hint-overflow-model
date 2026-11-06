# cortex9-rust-zip-size-hint-overflow-model

[English](README.md) · [Guía práctica](PRACTICE.md)

Modelo en Rust seguro (`#![forbid(unsafe_code)]`) de la especialización
`TrustedRandomAccess` del iterador `Zip` de `core`, antes y después del PR
#82289 de rust-lang/rust, que corrigió CVE-2021-28879.

**Es un modelo, no la biblioteca estándar ni una reproducción.** Índices y
longitudes son valores del modelo con tamaño de palabra elegible (8/16/32/64
bits). `__iterator_get_unchecked` se reemplaza por un evento registrado y
clasificado (dentro/fuera de contrato y de la asignación); nunca se lee
fuera de límites y no se usa ningún toolchain viejo o vulnerable. Ver
[LIMITATIONS.md](LIMITATIONS.md).

## El mecanismo

En la rama con efectos laterales de `next()` se hacía `index += 1` sin
`len += 1`; `size_hint() = len - index` desborda hacia abajo y un `Zip` de
esos zips confía en ese tamaño para llamar `get_unchecked` más allá de los
datos. El PR agrega `self.len += 1` y corrige `next_back`; su test
`test_issue_82282` es el escenario 01.

## Ejecutar

```sh
cargo test
cargo run --release --bin zip-model -- matrix fixtures/scenarios /tmp/out
diff -r results /tmp/out
```

## Resultados

- 01 vulnerable + wrapping: `size_hint` = 2^W − 1; 16 de 22 llamadas caen
  fuera de la asignación (10 en el ZST, 6 en el arreglo `i32`), contadas,
  ninguna realizada.
- 01 vulnerable + checked: se detiene en el primer desborde.
- 01 corregido: `size_hint` 0 y ningún ítem, como el test upstream.
- 02 (benigno) idéntico en ambas variantes; la variante corregida no tiene
  desbordes ni accesos fuera de contrato en ningún escenario.

Licencia: Apache-2.0.
