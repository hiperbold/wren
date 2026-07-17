# Roteiro — ditado-mensagem

**Cenário:** mensagem informal (chat/e-mail) com interrogação e exclamação —
as marcas que mais somem no ditado. ~12 s de fala + ~10 s de pausas ≈ 22 s.

**Como gravar:** `./scripts/record-golden.sh ditado-mensagem` — leia com a
entonação natural de pergunta/exclamação (a prosódia é a pista do modelo).
Nas marcações `⏸`, fique em silêncio pelo tempo indicado. Errou? Ctrl+C e
grave de novo.

---

> Oi, tudo bem? **⏸ 3 s** Consegui terminar aquele relatório ontem à noite.
> Ficou ótimo! **⏸ 4 s** Você consegue revisar até **⏸ 3 s** quinta-feira, ou
> prefere que eu mande direto para o cliente?

---

**O que cada pausa testa:**

- `tudo bem? ⏸ 3s Consegui` — a **interrogação** precisa sobreviver à pausa.
- `ótimo! ⏸ 4s Você` — a **exclamação** (marca mais rara) antes de pausa.
- `revisar até ⏸ 3s quinta-feira` — pausa no meio da oração, colada num
  substantivo: **nenhuma pontuação** deve aparecer, e "quinta-feira" não pode
  virar início de frase (capitalização).
