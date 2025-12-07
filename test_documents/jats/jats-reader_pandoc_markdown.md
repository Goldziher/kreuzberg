This is a set of tests for pandoc. Most of them are adapted from John
Gruber\'s markdown test suite.

# Headers

## Level 2 with an [embedded link](/url)

### Level 3 with *emphasis*

#### Level 4

##### Level 5

# Level 1

## Level 2 with *emphasis*

### Level 3

with no blank line

## Level 2

with no blank line

# Paragraphs

Here\'s a regular paragraph.

In Markdown 1.0.0 and earlier. Version 8. This line turns into a list
item. Because a hard-wrapped line in the middle of a paragraph looked
like a list item.

Here\'s one with a bullet. \* criminey.

There should be a **hard line break\**
here.

# Statements

## CAP TITLE

Some text to make this regular

# Block Quotes

E-mail style:

> This is a block quote. It is pretty short.

> Code in a block quote:
>
>     sub status {
>         print "working";
>     }
>
> A list:
>
> 1.  item one
>
> 2.  item two
>
> Nested block quotes:
>
> > nested
>
> > nested

This should not be a block quote: 2 \> 1.

Box-style:

> Example:
>
>     sub status {
>         print "working";
>     }

> 1.  do laundry
>
> 2.  take out the trash

Here\'s a nested one:

> Joe said:
>
> > Don\'t quote me.

And a following paragraph.

Here is a block quote inside a paragraph. Start of a paragraph is here,
and the quote is below:

> My block quote

End of a paragraph

# Formulae

## Inline formulae with *inline-formula* {#inline-formulae}

**Inside a paragraph**, should be natively coded inside the paragraph,
and show inline:

Einstein showed that $e=mc^2$. This formula is important because...

## Block formulae with *disp-formula* {#disp-formulae}

**Inside a paragraph**, should be natively coded inside the paragraph,
but show as block:

Einstein showed that $$e=mc^2$$. This formula is important because...

**Outside a paragraph**, should show as block:

Einstein showed that

::: disp-formula
$$e=mc^2$$
:::

This formula is important because...

## Without a formula wrapper {#no-formula-wrapper}

**Inside a paragraph**, should show inline:

Einstein showed that $e=mc^2$. This formula is important because...

**Outside a paragraph**, should show as block:

Einstein showed that

$$e=mc^2$$

This formula is important because...

## Formulae with miscelaneus elements {#misc-formulae}

Regardless of whether or not they are inside a paragraph, should be
wrapped in a div, and displayed in a block:

**Inside a paragraph:**

Einstein definitely showed that

::: disp-formula
Abstract text

![Alternative text of the
graphic](https://lh3.googleusercontent.com/dB7iirJ3ncQaVMBGE2YX-WCeoAVIChb6NAzoFcKCFChMsrixJvD7ZRbvcaC-ceXEzXYaoH4K5vaoRDsUyBHFkpIDPnsn3bnzovbvi0a2Gg=s660 "This is the title of the graphic"){#graphic001
.This .is .the .role .of .the .graphic}

$$e=mc^2$$
:::

. This formula is important because...

**Outside of a paragraph:**

Paragraph before: Einstein showed that

::: disp-formula
Abstract text

![Alternative text 1](Title 3){#graphic003}

$$e=mc^2$$
:::

This formula is important because...

<figure id="fig-1">
alternative-decription
<p><img src="foo.png" /></p>
<figcaption>bar</figcaption>
</figure>

# Code Blocks

Code:

    ---- (should be four hyphens)

    sub status {
        print "working";
    }

    this code block is indented by one tab

And:

        this code block is indented by two tabs

    These should not be escaped:  \$ \\ \> \[ \{

# Lists

## Unordered

Asterisks tight:

- asterisk 1

- asterisk 2

- asterisk 3

Asterisks loose:

- asterisk 1

- asterisk 2

- asterisk 3

Pluses tight:

- Plus 1

- Plus 2

- Plus 3

Pluses loose:

- Plus 1

- Plus 2

- Plus 3

Minuses tight:

- Minus 1

- Minus 2

- Minus 3

Minuses loose:

- Minus 1

- Minus 2

- Minus 3

## Ordered

Tight:

1.  First

2.  Second

3.  Third

and:

1.  One

2.  Two

3.  Three

Loose using tabs:

1.  First

2.  Second

3.  Third

and using spaces:

1.  One

2.  Two

3.  Three

Multiple paragraphs:

1.  Item 1, graf one.

    Item 1. graf two. The quick brown fox jumped over the lazy dog\'s
    back.

2.  Item 2.

3.  Item 3.

List styles:

1.  one

2.  two

3.  three

<!-- -->

i.  one

ii. two

iii. three

## Nested

- Tab

  - Tab

    - Tab

Here\'s another:

1.  First

2.  Second:

    - Fee

    - Fie

    - Foe

3.  Third

Same thing but with paragraphs:

1.  First

2.  Second:

    - Fee

    - Fie

    - Foe

3.  Third

## Tabs and spaces

- this is a list item indented with tabs

- this is a list item indented with spaces

  - this is an example list item indented with tabs

  - this is an example list item indented with spaces

## Fancy list markers

Autonumbering:

1.  Autonumber.

2.  More.

    1.  Nested.

## Definition

Violin

:   Stringed musical instrument.

    Torture device.

Cello**\**
Violoncello

:   Low-voiced stringed instrument.

## List inside a paragraph {#list-inside-paragraph}

Start of a paragraph.

- Red

- Blue

End of paragraph.

# Inline Markup

This is *emphasized*, and so *is this*.

This is **strong**, and so **is this**.

Empty and .

An *[emphasized link](/url)*.

***This is strong and em.***

So is ***this*** word.

***This is strong and em.***

So is ***this*** word.

This is code: `>`, `$`, `\`, `\$`, `<html>`.

This is [small caps]{.smallcaps}.

These are all underlined: foo and bar.

These are all strikethrough: ~~foo~~, ~~bar~~, and ~~baz~~.

# Smart quotes, ellipses, dashes

\"Hello,\" said the spider. \"\'Shelob\' is my name.\"

\'A\', \'B\', and \'C\' are letters.

\'Oak,\' \'elm,\' and \'beech\' are names of trees. So is \'pine.\'

\'He said, \"I want to go.\"\' Were you alive in the 70\'s?

Here is some quoted \'`code`\' and a \"[quoted
link](http://example.com/?foo=1&bar=2)\".

Some dashes: one\-\--two \-\-- three\--four \-- five.

Dashes between numbers: 5-7, 255-66, 1987-1999.

Ellipses\...and. . .and . . . .

# LaTeX

- \\cite\[22-23\]{smith.1899}

- \\doublespacing

- \$2+2=4\$

- \$x \\in y\$

- \$\\alpha \\wedge \\omega\$

- \$223\$

- \$p\$-Tree

- \$\\frac{d}{dx}f(x)=\\lim\_{h\\to 0}\\frac{f(x+h)-f(x)}{h}\$

- Here\'s one that has a line break in it: \$\\alpha + \\omega \\times
  x\^2\$.

These shouldn\'t be math:

- To get the famous equation, write `$e = mc^2$`.

- \$22,000 is a *lot* of money. So is \$34,000. (It worked if \"lot\" is
  emphasized.)

- Escaped `$`: \$73 *this should be emphasized* 23\$.

Here\'s a LaTeX table:

\\begin{tabular}{\|l\|l\|}\\hline Animal & Number \\\\ \\hline Dog & 2
\\\\ Cat & 1 \\\\ \\hline \\end{tabular}

# Special Characters

Here is some unicode:

- I hat: Î

- o umlaut: ö

- section: §

- set membership: elem

- copyright: ©

AT&T has an ampersand in their name.

AT&T is another way to write it.

This & that.

4 \< 5.

6 \> 5.

Backslash: \\

Backtick: \`

Asterisk: \*

Underscore: \_

Left brace: {

Right brace: }

Left bracket: \[

Right bracket: \]

Left paren: (

Right paren: )

Greater-than: \>

Hash: \#

Period: .

Bang: !

Plus: +

Minus: -

# Links

## Explicit

Just a [URL](/url/).

[URL and title](/url/ "title").

[URL and title](/url/ "title preceded by two spaces").

[URL and title](/url/ "title preceded by a tab").

[URL and title](/url/ "title with "quotes" in it")

[URL and title](/url/ "title with single quotes")

Email link (nobody \[at\] nowhere.net)

[Empty]().

## Reference

Foo [bar](/url/).

Foo [bar](/url/).

Foo [bar](/url/).

With [embedded \[brackets\]](/url/).

[b](/url/) by itself should be a link.

Indented [once](/url).

Indented [twice](/url).

Indented [thrice](/url).

This should \[not\] be a link.

    [not]: /url

Foo [bar](/url/ "Title with "quotes" inside").

Foo [biz](/url/ "Title with "quote" inside").

## With ampersands

Here\'s a [link with an ampersand in the
URL](http://example.com/?foo=1&bar=2).

Here\'s a link with an amersand in the link text:
[AT&T](http://att.com/ "AT&T").

Here\'s an [inline link](/script?foo=1&bar=2).

Here\'s an [inline link in pointy braces](/script?foo=1&bar=2).

## Autolinks

With an ampersand: <http://example.com/?foo=1&bar=2>

- In a list?

- <http://example.com/>

- It should.

An e-mail address: nobody \[at\] nowhere.net

> Blockquoted: <http://example.com/>

Auto-links should not occur here: `<http://example.com/>`

    or here: <http://example.com/>

# Images

From \"Voyage dans la Lune\" by Georges Melies (1902):

![](lalune.jpg "Voyage dans la Lune")

Here is a movie ![](movie.jpg) icon.

# Footnotes

Here is a footnote reference[(1)](#note_1), and
another[(longnote)](#note_longnote). This should *not* be a footnote
reference, because it contains a space\^(my note).

[(1)](#ref_1) Here is the footnote. It can go anywhere in the document,
not just at the end.

[(longnote)](#ref_longnote) Here\'s the other note. This one contains
multiple blocks.

Caret characters are used to indicate that the blocks all belong to a
single footnote (as with block quotes).

      { <code> }

If you want, you can use a caret at the beginning of every line, as with
blockquotes, but all that you need is a caret at the beginning of the
first line of the block and any preceding blank lines.

text *Leading space*

*Trailing space* text

text *Leading spaces*

*Trailing spaces* text

# Tables

## Tables with Headers

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

::: table-wrap
  r1a   r1b   r1c
  ----- ----- -----
  X     Y     Z
  1     2     3
  4     5     6
:::

::: table-wrap
  X   Y   Z
  --- --- ---
  1   2   3
  4   5   6
:::

## Tables without Headers

::: table-wrap
  --- --- ---
  1   2   3
  4   5   6
  --- --- ---
:::

::: table-wrap
  --- --- ---
  1   2   3
  4   5   6
  --- --- ---
:::

::: table-wrap
  --- --- ---
  1   2   3
  4   5   6
  --- --- ---
:::

::: table-wrap
  --- --- ---
  1   2   3
  4   5   6
  --- --- ---
:::

## Tables with spans and alignments {#table-with-spans-and-alignments}

::: table-wrap
+-------+---+
| 1     | 2 |
+:==+:==+:==+
| 1     | 2 |
+---+---+---+
| 4 | 5 | 6 |
|   +---+---+
|   | 7     |
+---+-------+
:::

## Table with footer

::: table-wrap
  ---- ---- ----
  1    2    3
  f1   f2   f3
  f4   f5   f6
  ---- ---- ----
:::

## Table With Multiple Bodies

::: table-wrap
  ---- ---- ----
  a1   a2   a3
  b1   b2   b3
  ---- ---- ----
:::

## Empty Tables

This section should be empty.

# References

::: {#refs}
:::

# Notes
