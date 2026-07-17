# Roteiro — ditado-prompt

**Cenário:** ditar uma instrução para uma IA (o uso real do Wren), com pausas
de raciocínio. ~15 s de fala + ~12 s de pausas ≈ 27 s.

**Como gravar:** `./scripts/record-golden.sh ditado-prompt` — leia o texto
abaixo em voz natural de ditado. Nas marcações `⏸`, **fique em silêncio** pelo
tempo indicado (não fale "pausa", não tussa, só espere). Se errar uma palavra,
pare (Ctrl+C) e grave de novo — a referência precisa bater exatamente.

---

> Preciso que você refatore o módulo de configurações para separar a leitura
> da escrita. **⏸ 4 s** Depois disso, rode os testes e **⏸ 3 s** me mostre um
> resumo do que mudou. **⏸ 5 s** Se algum teste quebrar, não tente corrigir
> sozinho, só me avise.

---

**O que cada pausa testa:**

- `escrita. ⏸ 4s Depois` — pausa em fronteira de frase: o **ponto final**
  precisa sobreviver à compressão.
- `testes e ⏸ 3s me mostre` — pausa NO MEIO da oração: **nenhuma pontuação**
  deve aparecer aqui (o resíduo não pode virar ponto).
- `mudou. ⏸ 5s Se algum` — pausa longa (acima do limiar de 2 s com folga) em
  fronteira de frase.
