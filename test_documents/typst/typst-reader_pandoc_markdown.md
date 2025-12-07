# Fibonacci sequence

The Fibonacci sequence is defined through the recurrence relation
$F_{n} = F_{n - 1} + F_{n - 2}$. It can also be expressed in *closed
form:*

$$F_{n} = \left\lfloor {\frac{1}{\sqrt{5}}\phi^{n}} \right\rceil,\quad\phi = \frac{1 + \sqrt{5}}{2}$$

The first 8 numbers of the sequence are:

::: {align="center"}
  --------- --------- --------- --------- --------- --------- --------- ---------
  $F_{1}$   $F_{2}$   $F_{3}$   $F_{4}$   $F_{5}$   $F_{6}$   $F_{7}$   $F_{8}$
  1         1         2         3         5         8         13        21
  --------- --------- --------- --------- --------- --------- --------- ---------
:::

::::::::::::::::::::::: {.columns-flow count="2"}
[[ **Typst Math for Undergrads**
](https://github.com/johanvx/typst-undergradmath)]{align="center"}

This is a Typst port of *[L A [T E X]{.box}]{.box} Math for Undergrads*
by Jim Hefferon. The original version is available at
[<https://gitlab.com/jim.hefferon/undergradmath>]{.underline}.

**Meaning of annotations  **

+:----------------------+:----------------------------------------------+
| [2023-05-22 ❌]{.box} | This is unavailable. Last check date is       |
|                       | 2023-05-22.                                   |
+-----------------------+-----------------------------------------------+

[]{#unavailable}

+:----------------------+:----------------------------------------------+
| [💦]{.box}            | Get this in a tricky way. Need a simpler      |
|                       | method.                                       |
+-----------------------+-----------------------------------------------+

[]{#tricky}

+:----------------------+:----------------------------------------------+
| [No idea 😕]{.box}    | Don't know how to get this.                   |
+-----------------------+-----------------------------------------------+

[]{#noidea}

**Rule One  **Any mathematics at all, even a single character, gets a
mathematical setting. Thus, for "the value of $x$ is $7$" enter
`the value of $x$ is $7$`.

**Template  **Your document should contain at least this.

+---+------------------------------+
|   |     -- document body here -- |
|   |                              |
+---+------------------------------+

**Common constructs  **

::: {align="center"}
  ------------------------------ ---------------------------------------------------------------
  [$x^{2}$ `x^2`]{.box}          [$\sqrt{2}$, $\sqrt[n]{3}$ `sqrt(2)`, `root(n, 3)`]{.box}
  [$x_{i,j}$ `x_(i, j)`]{.box}   [$\frac{2}{3}$, $2/3$ `2 / 3`, `2 \/ 3` or `2 slash 3`]{.box}
  ------------------------------ ---------------------------------------------------------------
:::

**Calligraphic letters  **Use as in `$cal(A)$`.

$$\mathcal{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$$

Getting script letters is [\[unavailable\]](#unavailable){.ref}.

**Greek  **

::: {align="center"}
  ------------------------------------------------- ------------------------------------------------------------
  [$\alpha$ `alpha`]{.box}                          [$\xi$, $\Xi$ `xi`, `Xi`]{.box}
  [$\beta$ `beta`]{.box}                            [$ο$ `omicron`]{.box}
  [$\gamma$, $\Gamma$ `gamma`, `Gamma`]{.box}       [$\pi$, $\Pi$ `pi`, `Pi`]{.box}
  [$\delta$, $\Delta$ `delta`, `Delta`]{.box}       [$\varpi$ `pi.alt`]{.box}
  [$\epsilon$ `epsilon.alt`]{.box}                  [$\rho$ `rho`]{.box}
  [$\varepsilon$ `epsilon`]{.box}                   [$\varrho$ `rho.alt`]{.box}
  [$\zeta$ `zeta`]{.box}                            [$\sigma$, $\Sigma$ `sigma`, `Sigma`]{.box}
  [$\eta$ `eta`]{.box}                              [$\varsigma$ `\u{03C2}` [\[tricky\]](#tricky){.ref}]{.box}
  [$\theta$, $\Theta$ `theta`, `Theta`]{.box}       [$\tau$ `tau`]{.box}
  [$\vartheta$ `theta.alt`]{.box}                   [$\upsilon$, $\Upsilon$ `upsilon`, `Upsilon`]{.box}
  [$\iota$ `iota`]{.box}                            [$\phi$, $\Phi$ `phi.alt`, `Phi`]{.box}
  [$\kappa$ $Κ$]{.box}                              [$\varphi$ `phi`]{.box}
  [$\lambda$, $\Lambda$ `lambda`, `Lambda`]{.box}   [$\chi$ `chi`]{.box}
  [$\mu$ `mu`]{.box}                                [$\psi$, $\Psi$ `psi`, `Psi`]{.box}
  [$\nu$ `nu`]{.box}                                [$\omega$, $\Omega$ `omega`, `Omega`]{.box}
  ------------------------------------------------- ------------------------------------------------------------
:::

**Sets and logic  **

::: {align="center"}
  --------------------------------- ------------------------------------ ------------------------------------
  [$\cup$ `union`]{.box}            [$\mathbb{R}$ `RR`, `bb(R)`]{.box}   [$\forall$ `forall`]{.box}
  [$\cap$ `sect`]{.box}             [$\mathbb{Z}$ `ZZ`, `bb(Z)`]{.box}   [$\exists$ `exists`]{.box}
  [$\subset$ `subset`]{.box}        [$\mathbb{Q}$ `QQ`, `bb(Q)`]{.box}   [$\neg$ `not`]{.box}
  [$\subseteq$ `subset.eq`]{.box}   [$\mathbb{N}$ `NN`, `bb(N)`]{.box}   [$\vee$ `or`]{.box}
  [$\supset$ `supset`]{.box}        [$\mathbb{C}$ `CC`, `bb(C)`]{.box}   [$\land$ `and`]{.box}
  [$\supseteq$ `supset.eq`]{.box}   [$\varnothing$ `diameter`]{.box}     [$\vdash$ `tack.r`]{.box}
  [$\in$ `in`]{.box}                [$\varnothing$ `nothing`]{.box}      [$\models$ `models`]{.box}
  [$\notin$ `in.not`]{.box}         [$א$ `alef`]{.box}                   [$\smallsetminus$ `without`]{.box}
  --------------------------------- ------------------------------------ ------------------------------------
:::

Negate an operator, as in $⊄$, with `subset.not`. Get the set complement
$A^{\mathsf{c}}$ with `A^(sans(c))` (or $A^{\complement}$ with
`A^(complement)`, or $\overline{A}$ with `overline(A)`).

Remark

:   Using `diameter` for `\varnothing` may cause some confusion.
    However, [L A [T E X]{.box}]{.box} also uses $\varnothing$
    (`\u{2300}`) instead of $\varnothing$ (`\u{2205}`), see [[newcm
    $§$`<!-- -->`{=html}13.3](https://mirrors.sustech.edu.cn/CTAN/fonts/newcomputermodern/doc/newcm-doc.pdf)]{.underline}.
    Another solution is to use `text(font: "Fira Sans", nothing)`, but
    the resultant glyph $\varnothing$ is subtly different from the
    widely used one. Ultimately, the choice is always **your decision**.

**Decorations  **

::: {align="center"}
  ----------------------------------- ------------------------------- -----------------------------------------------
  [$f'$ `f'`, `f prime`]{.box}        [$\dot{a}$ `dot(a)`]{.box}      [$\widetilde{a}$ `tilde(a)`]{.box}
  [$f''$ `f prime.double`]{.box}      [$\ddot{a}$ `diaer(a)`]{.box}   [$\overline{a}$ `macron(a)`]{.box}
  [$\Sigma^{\ast}$ `Sigma^*`]{.box}   [$\hat{a}$ `hat(a)`]{.box}      [$\overset{\rightarrow}{a}$ `arrow(a)`]{.box}
  ----------------------------------- ------------------------------- -----------------------------------------------
:::

If the decorated letter is $i$ or $j$ then some decorations need
`\u{1D6A4}` [\[tricky\]](#tricky){.ref} and `\u{1D6A5}`
[\[tricky\]](#tricky){.ref}, as in $\overset{\rightarrow}{\imath}$ with
`arrow(\u{1D6A4})`. Some authors use boldface for vectors: `bold(x)`.

Entering `overline(x + y)` produces $\overline{x + y}$, and `hat(x + y)`
gives $\hat{x + y}$. Comment on an expression as here (there is also
`overbrace(..)`).

[[$\underset{|A|}{\underbrace{x + y}}$ `underbrace(x + y, |A|)`]{.box}]{align="center"}

**Dots  **Use low dots in a list $\left\{ 0,1,2,\ldots \right\}$,
entered as `{0, 1, 2, ...}`. Use centered dots in a sum or product
$1 + \cdots + 100$, entered as `1 + dots.h.c + 100`. You can also get
vertical dots `dots.v`, diagonal dots `dots.down` and anti-diagonal dots
`dots.up`.

**Roman names  **Just type them!

::: {align="center"}
  ---------------------- ------------------------ ----------------------------
  [$\sin$ `sin`]{.box}   [$\sinh$ `sinh`]{.box}   [$\arcsin$ `arcsin`]{.box}
  [$\cos$ `cos`]{.box}   [$\cosh$ `cosh`]{.box}   [$\arccos$ `arccos`]{.box}
  [$\tan$ `tan`]{.box}   [$\tanh$ `tanh`]{.box}   [$\arctan$ `arctan`]{.box}
  [$\sec$ `sec`]{.box}   [$\coth$ `coth`]{.box}   [$\min$ `min`]{.box}
  [$\csc$ `csc`]{.box}   [$\det$ `det`]{.box}     [$\max$ `max`]{.box}
  [$\cot$ `cot`]{.box}   [$\dim$ `dim`]{.box}     [$\inf$ `inf`]{.box}
  [$\exp$ `exp`]{.box}   [$\ker$ `ker`]{.box}     [$\sup$ `sup`]{.box}
  [$\log$ `log`]{.box}   [$\deg$ `deg`]{.box}     [$\liminf$ `liminf`]{.box}
  [$\ln$ `ln`]{.box}     [$\arg$ `arg`]{.box}     [$\limsup$ `limsup`]{.box}
  [$\lg$ `lg`]{.box}     [$\gcd$ `gcd`]{.box}     [$\lim$ `lim`]{.box}
  ---------------------- ------------------------ ----------------------------
:::

**Other symbols  **

::: {align="center"}
  --------------------------------------------------------- ---------------------------------------------------------- ---------------------------------------
  [$<$ `<`, `lt`]{.box}                                     [$\angle$ `angle`]{.box}                                   [$\cdot$ `dot`]{.box}
  [$\leq$ `<=`, `lt.eq`]{.box}                              [$\measuredangle$ `angle.arc`]{.box}                       [$\pm$ `plus.minus`]{.box}
  [$>$ `>`, `gt`]{.box}                                     [$\ell$ `ell`]{.box}                                       [$\mp$ `minus.plus`]{.box}
  [$\geq$ `>=`, `gt.eq`]{.box}                              [$\parallel$ `parallel`]{.box}                             [$\times$ `times`]{.box}
  [$\neq$ `!=`, `eq.not`]{.box}                             [$45{^\circ}$ `45 degree`]{.box}                           [$\div$ `div`]{.box}
  [$\ll$ `<<`, `lt.double`]{.box}                           [$\cong$ `tilde.equiv`]{.box}                              [$\ast$ `*`, `ast`]{.box}
  [$\gg$ `>>`, `gt.double`]{.box}                           [$\ncong$ `tilde.equiv.not`]{.box}                         [$\mid$ `divides`]{.box}
  [$\approx$ `approx`]{.box}                                [$\sim$ `tilde`]{.box}                                     [$\nmid$ `divides.not`]{.box}
  [$\asymp$ `\u{224D}` [\[tricky\]](#tricky){.ref}]{.box}   [$\simeq$ `tilde.eq`]{.box}                                [$n!$ `n!`]{.box}
  [$\equiv$ `equiv`]{.box}                                  [$\nsim$ `tilde.not`]{.box}                                [$\partial$ `diff`]{.box}
  [$\prec$ `prec`]{.box}                                    [$\oplus$ `plus.circle`]{.box}                             [$\nabla$ `nabla`]{.box}
  [$\preceq$ `prec.eq`]{.box}                               [$\ominus$ `minus.circle`]{.box}                           [$ħ$ `planck.reduce`]{.box}
  [$\succ$ `succ`]{.box}                                    [$\odot$ `dot.circle`]{.box}                               [$\circ$ `circle.stroked.tiny`]{.box}
  [$\succeq$ `succ.eq`]{.box}                               [$\otimes$ `times.circle`]{.box}                           [$\star$ `star`]{.box}
  [$\propto$ `prop`]{.box}                                  [$\oslash$ `\u{2298}` [\[tricky\]](#tricky){.ref}]{.box}   [$\sqrt{}$ `sqrt("")`]{.box}
  [$\doteq$ `\u{2250}` [\[tricky\]](#tricky){.ref}]{.box}   [$\upharpoonright$ `harpoon.tr`]{.box}                     [$✓$ `checkmark`]{.box}
  --------------------------------------------------------- ---------------------------------------------------------- ---------------------------------------
:::

Use `a divides b` for the divides relation, $a \mid b$, and
`a divides.not b` for the negation, $a \nmid b$. Use `|` to get set
builder notation $\left\{ a \in S~|~a\text{ is odd} \right\}$ with
`{a in S | a "is odd"}`.

**Arrows  **

::: {align="center"}
  ------------------------------------------------- --------------------------------------------------------
  [$\rightarrow$ `->`, `arrow.r`]{.box}             [$\mapsto$ `|->`, `arrow.r.bar`]{.box}
  [$\nrightarrow$ `arrow.r.not`]{.box}              [$\longmapsto$ `arrow.r.long.bar`]{.box}
  [$\longrightarrow$ `arrow.r.long`]{.box}          [$\leftarrow$ `<-`, `arrow.l`]{.box}
  [$\Rightarrow$ `=>`, `arrow.r.double`]{.box}      [$\longleftrightarrow$ `<-->`, `arrow.l.r.long`]{.box}
  [$\nRightarrow$ `arrow.r.double.not`]{.box}       [$\downarrow$ `arrow.b`]{.box}
  [$\Longrightarrow$ `arrow.r.double.long`]{.box}   [$\uparrow$ `arrow.t`]{.box}
  [$\rightsquigarrow$ `arrow.squiggly`]{.box}       [$\updownarrow$ `arrow.t.b`]{.box}
  ------------------------------------------------- --------------------------------------------------------
:::

The right arrows in the first column have matching left arrows, such as
`arrow.l.not`, and there are some other matches for down arrows, etc.

**Variable-sized operators  **The summation $\sum_{j = 0}^{3}j^{2}$
`sum_(j = 0)^3 j^2` and the integral $\int_{x = 0}^{3}x^{2}dx$
`integral_(x = 0)^3 x^2 dif x` expand when displayed.

$$\sum_{j = 0}^{3}j^{2}\qquad\int_{x = 0}^{3}x^{2}dx$$

These do the same.

::: {align="center"}
  ----------------------------------- ------------------------------------ -------------------------------
  [$\int$ `integral`]{.box}           [$\iiint$ `integral.triple`]{.box}   [$\bigcup$ `union.big`]{.box}
  [$\iint$ `integral.double`]{.box}   [$\oint$ `integral.cont`]{.box}      [$\bigcap$ `sect.big`]{.box}
  ----------------------------------- ------------------------------------ -------------------------------
:::

**Fences  **

::: {align="center"}
  --------------------------------- ----------------------------------------------------- ------------------------------------------
  [$()$ `()`]{.box}                 [$\langle\rangle$ `angle.l angle.r`]{.box}            [$\left| {} \right|$ `abs("")`]{.box}
  [$\lbrack\rbrack$ `[]`]{.box}     [$\left\lfloor {} \right\rfloor$ `floor("")`]{.box}   [$\left\| {} \right\|$ `norm("")`]{.box}
  [$\left\{ \right\}$ `{}`]{.box}   [$\left\lceil {} \right\rceil$ `ceil("")`]{.box}
  --------------------------------- ----------------------------------------------------- ------------------------------------------
:::

Fix the size with the `lr` function.

::: {align="center"}
+:------------------------------------------------------------------------+:--------------------------------------------+
| $$\left. \left\lbrack \sum_{k = 0}^{n}e^{k^{2}} \right\rbrack \right.$$ |     lr([sum_(k = 0)^n e^(k^2)], size: #50%) |
|                                                                         |                                             |
+-------------------------------------------------------------------------+---------------------------------------------+
:::

To have them grow with the enclosed formula, also use the `lr` function.

::: {align="center"}
+:-------------------------------------------+:-----------------------------------+
| $$\left\langle i,2^{2^{i}} \right\rangle$$ |     lr(angle.l i, 2^(2^i) angle.r) |
|                                            |                                    |
+--------------------------------------------+------------------------------------+
:::

Fences scale by default if entered directly as codepoints, and don't
scale automatically if entered as symbol notation.

::: {align="center"}
+:----------------------------------------+:----------------------------------+
| $$\left( \frac{1}{n^{\alpha}} \right)$$ |     (1 / n^(alpha))               |
|                                         |                                   |
+-----------------------------------------+-----------------------------------+
| $$(\frac{1}{n^{\alpha}})$$              |     paren.l 1 / n^(alpha) paren.r |
|                                         |                                   |
+-----------------------------------------+-----------------------------------+
:::

The `lr` function also allows to scale unmatched delimiters and one-side
fences.

::: {align="center"}
+:-----------------------------------------+:-----------------------------------+
| $$\left. \frac{df}{dx} \right|_{x_{0}}$$ |     lr(frac(dif f, dif x) |)_(x_0) |
|                                          |                                    |
+------------------------------------------+------------------------------------+
:::

**Arrays, Matrices  **Get a matrix with the `mat` function. You can pass
an array to it.

::: {align="center"}
+:------------------+:------------------------+
| $$\begin{pmatrix} |     $ mat(a, b; c, d) $ |
| a & b \\          |                         |
| c & d             |                         |
| \end{pmatrix}$$   |                         |
+-------------------+-------------------------+
:::

In Typst,
[[array](https://typst.app/docs/reference/typst/array)]{.underline} is a
sequence of values, while in [L A [T E X]{.box}]{.box}, array is a
matrix without fences, which is `$mat(delim: #none, ..)$` in Typst.

For the determinant use `|A|`, text operator $\det$ `det` or
`mat(delim: "|", ..)`.

Definition by cases can be easily obtained with the `cases` function.

::: {align="center"}
+:---------------------------------+:--------------------------------+
| $$f_{n} = \begin{cases}          |     $ f_n = cases(              |
| a & \text{if }n = 0 \\           |         a &"if" n = 0,          |
| r \cdot f_{n - 1} & \text{else } |         r dot f_(n - 1) &"else" |
| \end{cases}$$                    |       ) $                       |
|                                  |                                 |
+----------------------------------+---------------------------------+
:::

**Spacing in mathematics  **Improve $\sqrt{2}x$ to $\sqrt{2}\, x$ with a
thin space, as in `sqrt(2) thin x`. Slightly wider are `medium` and
`thick` (the three are in ratio $3:4:5$). Bigger space is `quad` for
$\rightarrow \quad \leftarrow$, which is useful between parts of a
display. Get arbitrary space with the `h` function. For example, use
`#h(2em)` for `\qquad` in [L A [T E X]{.box}]{.box} and `#h(-0.1667em)`
for `\!`.

**Displayed equations  **Display equations in a block level using
`$ ... $` with at least one space separating the math content and the
`$`.

::: {align="center"}
+:----------------------+:-----------------------+
| $$S = k \cdot \lg W$$ |     $ S = k dot lg W $ |
|                       |                        |
+-----------------------+------------------------+
:::

You can break into multiple lines.

::: {align="center"}
+:----------------------------------+:----------------------------------+
| $$\begin{array}{r}                |     $ sin(x) = x - x^3 / 3! \     |
| \sin(x) = x - \frac{x^{3}}{3!} \\ |           + x^5 / 5! - dots.h.c $ |
|  + \frac{x^{5}}{5!} - \cdots      |                                   |
| \end{array}$$                     |                                   |
+-----------------------------------+-----------------------------------+
:::

Align equations using `&`

::: {align="center"}
+:------------------------------------+:---------------------------------+
| $$\begin{aligned}                   |     $ nabla dot bold(D) &= rho \ |
| \nabla \cdot \mathbf{D} & = \rho \\ |         nabla dot bold(B) &= 0 $ |
| \nabla \cdot \mathbf{B} & = 0       |                                  |
| \end{aligned}$$                     |                                  |
+-------------------------------------+----------------------------------+
:::

(the left or right side of an alignment can be empty). Get a numbered
version by `#set math.equation(numbering: ..)`.

**Calculus examples  **The last three here are display style.

::: {align="center"}
+---------------------------------------------------------------------------------------+-----------------------------------------------------------------------------------+
| $f:{\mathbb{R}} \rightarrow {\mathbb{R}}$                                             |     f: RR -> RR                                                                   |
|                                                                                       |                                                                                   |
+---------------------------------------------------------------------------------------+-----------------------------------------------------------------------------------+
| $9.8\ \text{ m/s}^{2}$                                                                | `"9.8" "m/s"^2` [\[tricky\]](#tricky){.ref}                                       |
+---------------------------------------------------------------------------------------+-----------------------------------------------------------------------------------+
| $$\lim\limits_{h \rightarrow 0}\frac{f(x + h) - f(x)}{h}$$                            |     lim_(h -> 0) (f(x + h) - f(x)) / h                                            |
|                                                                                       |                                                                                   |
+---------------------------------------------------------------------------------------+-----------------------------------------------------------------------------------+
| $$\int x^{2}dx = x^{3}/3 + C$$                                                        |     integral x^2 dif x = x^3 \/ 3 + C                                             |
|                                                                                       |                                                                                   |
+---------------------------------------------------------------------------------------+-----------------------------------------------------------------------------------+
| $$\nabla = \mathbf{i}\frac{d}{dx} + \mathbf{j}\frac{d}{dy} + \mathbf{k}\frac{d}{dz}$$ |     nabla = bold(i) dif / (dif x) + bold(j) dif / (dif y) + bold(k) dif / (dif z) |
|                                                                                       |                                                                                   |
+---------------------------------------------------------------------------------------+-----------------------------------------------------------------------------------+
:::

**Discrete mathematics examples  **For modulo, there is a symbol
$\equiv$ from `equiv` and a text operator $\operatorname{mod}$ from
`mod`.

For combinations the binomial symbol $\binom{n}{k}$ is from
`binom(n, k)`. This resizes to be bigger in a display.

For permutations use $n^{\underline{r}}$ from `n^(underline(r))` (some
authors use $P(n,r)$, or ${}_{n}P_{r}$ from `""_n P_r`).

**Statistics examples  **

::: {align="center"}
+--------------------------------------------------------+------------------------------------------+
| $\sigma^{2} = \sqrt{{\sum(x_{i} - \mu)}^{2}/N}$        |     sigma^2 = sqrt(sum(x_i - mu)^2 \/ N) |
|                                                        |                                          |
+--------------------------------------------------------+------------------------------------------+
| $E(X) = \mu_{X} = \sum(x_{i} - P\left( x_{i} \right))$ |     E(X) = mu_X = sum(x_i - P(x_i))      |
|                                                        |                                          |
+--------------------------------------------------------+------------------------------------------+
:::

The probability density of the normal distribution

$$\frac{1}{\sqrt{2\sigma^{2}\pi}}e^{- \frac{(x - \mu)^{2}}{2\sigma^{2}}}$$

comes from this.

+---+----------------------------------------+
|   |     1 / sqrt(2 sigma^2 pi)             |
|   |         e^(- (x - mu)^2 / (2 sigma^2)) |
|   |                                        |
+---+----------------------------------------+

**For more  **See also the Typst Documentation at
[<https://typst.app/docs>]{.underline}.

------------------------------------------------------------------------

johanvx ([<https://github.com/johanvx>]{.underline})    2023-05-22
:::::::::::::::::::::::
