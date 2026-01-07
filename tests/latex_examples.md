# Exemple de rendu LaTeX

Ce fichier montre plusieurs formules LaTeX que `markdown2pdf` peut rendre.

Affichage (display math):

$$
H(s) = \prod_{i=1}^{n/2} \frac{1}{s^2 + \frac{\omega_0}{Q_i}s + \omega_0^2}
$$

$$
\Delta f = \frac{f_s}{N}
$$

$$
\Delta f = \frac{48000}{4096} \approx 11.7\ \text{Hz}
$$

$$
f_{peak} = f_k + \frac{\delta f}{2} \cdot \frac{m_{k-1} - m_{k+1}}{m_{k-1} - 2m_k + m_{k+1}}
$$

$$
C = a_0 + a_1 \cdot S + a_2 \cdot S^2 + a_3 \cdot S^3 + a_4 \cdot S^4
$$

Exemple d'équation inline: $\Delta f = \frac{f_s}{N}$.

---

Pour générer un PDF localement :

```bash
./target/debug/markdown2pdf -p tests/latex_examples.md -o latex_example.pdf
```

Note: la génération de SVG via MicroTeX peut être lente et nécessite les dépendances natives configurées sur votre machine. Si la commande échoue, exécutez le test d'intégration ignoré avec :

```bash
cargo test --test latex_render_integration -- --ignored
```
