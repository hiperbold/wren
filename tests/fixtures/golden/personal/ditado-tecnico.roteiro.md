# Roteiro — ditado-tecnico

**Cenário:** ditado com vocabulário técnico (Wren, Tauri, Rust, wgpu, Groq) —
mede como os nomes próprios saem HOJE, e vira o baseline do glossário via
`prompt` da Fase 4. ~14 s de fala + ~8 s de pausas ≈ 22 s.

**Como gravar:** `./scripts/record-golden.sh ditado-tecnico` — pronuncie os
termos como você fala no dia a dia (uóren/táuri/rânst… vale o SEU jeito; o
teste é justamente esse). Nas marcações `⏸`, silêncio pelo tempo indicado.

---

> O Wren usa Tauri com o núcleo em Rust e a interface em React. **⏸ 5 s** O
> overlay nativo desenha a pílula de gravação com wgpu, e a transcrição sai
> pela **⏸ 3 s** API do Groq. Nada de runtime de machine learning dentro do
> app base.

---

**O que cada pausa testa:**

- `React. ⏸ 5s O overlay` — fronteira de frase com pausa bem acima do limiar.
- `sai pela ⏸ 3s API do Groq` — pausa no meio da oração, imediatamente antes
  de uma sigla + nome próprio: nenhuma pontuação, e "API"/"Groq" precisam
  sair grafados certos mesmo depois do resíduo de silêncio.
- Termos técnicos fora de pausa (Wren, Tauri, Rust, React, wgpu) — erros aqui
  são vocabulário, não pipeline: é o antes/depois do glossário da Fase 4.
