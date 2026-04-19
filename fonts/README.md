# Fonts (free / open source)

`build-all-and-run.sh` runs **`scripts/fetch-fos-fonts.sh`**, which downloads a large curated set of **SIL OFL**, **Apache 2.0**, **UFL**, and **DejaVu / Bitstream-derived** fonts into **`fonts/stratos/`** (gitignored binaries) and copies them into the minimal rootfs at **`/usr/share/fonts/stratos/`**.

**Refresh or pre-populate without a full build:**

```sh
./scripts/fetch-fos-fonts.sh          # skip files that already exist
./scripts/fetch-fos-fonts.sh --force  # re-download everything
```

**Included families (high level):** Noto Sans/Serif/Mono/Display, Arimo/Tinos/Cousine (metric-friendly), Roboto, Open Sans, Fira Sans/Mono, Lato, Montserrat/Oswald/Raleway/Nunito variable fonts, Merriweather/Playfair/Work Sans/DM Sans/Karla/Rubik/Bitter/EB Garamond/Crimson Pro/Quicksand/Manrope/Archivo/Lora/Fira Code, Space Mono, Anonymous Pro, Courier Prime, Slabo 27px, Bebas Neue, PT Sans/Serif, Cantarell, Ubuntu + Ubuntu Mono + Ubuntu Sans VF, Poppins, Spectral, Zilla Slab, **DejaVu**, **Inter** variable, **JetBrains Mono** (+ variable when present).

**Licensing:** Upstream packages carry their own `OFL.txt` / `LICENSE.txt` where applicable; see **`fonts/stratos/NOTICE.txt`** after a fetch. These are **not** Microsoft Arial/Times New Roman (proprietary); metric-close alternatives include **Arimo** / **Tinos** / **Cousine** and **Liberation-style** coverage via the Noto-derived set above.
