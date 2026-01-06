# LaTeX Mathematics Support in markdown2pdf

markdown2pdf now supports both **inline** and **display** LaTeX mathematical expressions using the lightweight MicroTeX renderer.

## Inline Mathematics

Inline math expressions are wrapped in single dollar signs: `$...$`

Examples in text:
- Einstein's mass-energy equivalence: $E = mc^2$
- Pythagorean theorem: $a^2 + b^2 = c^2$
- Quadratic formula: $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$

## Display Mathematics

Display math expressions are wrapped in double dollar signs: `$$...$$` and appear as block-level elements.

For example:

$$\int_0^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}$$

The Gaussian integral is one of the most important integrals in mathematics.

## Complex Examples

### Summation

$$\sum_{i=1}^{n} i^2 = \frac{n(n+1)(2n+1)}{6}$$

### Matrix Notation

$$\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{pmatrix} x \\ y \end{pmatrix} = \begin{pmatrix} ax + by \\ cx + dy \end{pmatrix}$$

### Calculus

The limit definition of derivative:

$$f'(x) = \lim_{h \to 0} \frac{f(x+h) - f(x)}{h}$$

## Mixed Content

You can freely mix LaTeX with regular markdown:

- **Bold text** and $\sigma$ symbol
- `Code blocks` and inline math $\lambda$
- [Links](https://example.com) with formulas: $\Phi = B \cdot A$

## Implementation Notes

- Inline math is displayed as readable text with code-style formatting (genpdfi_extended doesn't support inline SVG embedding)
- Display math is rendered as centered block-level elements
- If LaTeX rendering fails, fallback text is displayed instead
- Both `$...$` and `$$...$$` delimiters are automatically parsed and recognized
