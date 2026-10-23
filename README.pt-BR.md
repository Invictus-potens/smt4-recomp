<p align="center">
  <img src="https://github.com/user-attachments/assets/52714f2d-2c83-4e90-9bc9-e973ace2442f" alt="Shin Megami Tensei IV" width="117" height="108">
</p>

# Shin Megami Tensei IV para PC (smt4-recomp)

[English](README.md) | **Português (Brasil)**

Uma versão nativa para Windows e macOS de *Shin Megami Tensei IV* (3DS, EUA), feita
por recompilação estática: o [3dsrecomp](https://github.com/fearkov/3dsrecomp)
traduz o código ARM do jogo para C antes de rodar, e esse C é compilado num
programa próprio, com o [Zakuro](https://github.com/fearkov/zakuro) como
runtime para todo o resto (kernel, serviços, GPU, som). O código do jogo roda
nativamente, não pelo interpretador ou JIT de um emulador.

Além disso, o projeto adiciona widescreen, multiplicadores de Macca, EXP e
App Points, opções de frame rate e cheats, tudo ligado pelo menu de cheats
dentro do jogo.

> **Beta.** Jogado de um jogo novo por mais de 10 horas, sem travamentos.
> O resto do jogo ainda não foi jogado nele: relate o que encontrar em
> [Issues](../../issues).

> **Nenhum arquivo ou código do jogo está incluído, e não há download.** Você
> compila a partir do seu próprio dump do jogo. O que a compilação gera
> (`code/` e o programa) contém o código do jogo: não compartilhe.
> Este projeto não tem ligação com a Atlus, a SEGA ou a Nintendo.

## Conteúdo

- [Recursos](#recursos)
- [Requisitos](#requisitos)
- [Compilando](#compilando)
- [Jogando](#jogando)
- [Cheats e melhorias](#cheats-e-melhorias)
- [Controles](#controles)
- [Extraindo assets](#extraindo-assets)
- [Problemas comuns](#problemas-comuns)
- [Como funciona](#como-funciona)
- [Créditos](#créditos)
- [Licença](#licença)

## Recursos

- **Código nativo**: o código ARM do jogo recompilado para C e compilado
  para o PC (95% dele encontrado de antemão; o que sobra roda no
  interpretador do Zakuro).
- **Widescreen** 16:9 e 21:9: o jogo desenha uma cena 3D mais larga (mais
  visão nas laterais, mesma altura), não uma imagem esticada.
- **Multiplicadores** x2, x3 e x5 de Macca, EXP do herói, EXP dos demônios
  e App Points, escolhidos no menu de cheats durante o jogo.
- **Easy Recruit**: as negociações com demônios sempre dão certo, pelo menu
  de cheats.
- **60 FPS** (o jogo roda duas vezes mais rápido) e **30 FPS** para voltar.
- Cheats da comunidade: Macca no máximo, App Points, itens, EXP, HP/MP e
  atributos do herói.
- Tudo o que o Zakuro tem: Vulkan (Metal pelo MoltenVK no macOS) ou
  OpenGL, layouts de tela, escala e filtros, save states, controles, 3D.

## Requisitos

- Windows 10 ou 11, 64 bits. No Linux deve funcionar, mas não foi testado.
- Placa de vídeo com Vulkan (ou OpenGL 3.3).
- **Seu próprio dump descriptografado** de *Shin Megami Tensei IV* (EUA),
  title ID `00040000000E5C00`, em `.3ds` ou `.cci`.
- Uns 10 GB livres em disco e 8 GB de RAM para a compilação.
- Para compilar:
  - [Rust](https://rustup.rs) com o toolchain GNU:
    `rustup default stable-x86_64-pc-windows-gnu`
  - MinGW-w64 GCC, 64 bits: `winget install BrechtSanders.WinLibs.POSIX.UCRT`
  - [git](https://git-scm.com)

### macOS

- Um Mac com Apple Silicon (M1 ou mais novo). Macs com Intel não foram
  testados.
- As ferramentas de linha de comando da Apple (clang, ar e git):
  `xcode-select --install`
- [Rust](https://rustup.rs), o toolchain padrão
- [MoltenVK](https://github.com/KhronosGroup/MoltenVK), Vulkan sobre Metal,
  com que o jogo desenha: `brew install molten-vk` ([Homebrew](https://brew.sh)).
  A versão para macOS não tem OpenGL.

## Compilando

### Com o launcher

O `launcher/` é um programa pequeno que faz tudo por uma janela: escolha a
pasta com o seu jogo (ele acha a ROM nela), veja quais ferramentas faltam e
instale pelo winget, baixe ou atualize este projeto, compile o jogo com uma
barra de progresso e jogue.

```
cd launcher
cargo build --release
target\release\smt4-launcher.exe
```

No macOS ele é `target/release/smt4-launcher`, e instala as ferramentas com
`xcode-select --install`, o rustup e `brew install molten-vk`.

Ele não contém código do jogo, então pode ser compartilhado.

### Pela linha de comando

```
git clone https://github.com/Invictus-potens/smt4-recomp
cd smt4-recomp
cargo build-game "C:\caminho\para\Shin Megami Tensei IV (USA).3ds"
```

O `cargo build-game` (o builder em `tools/build`) confere suas ferramentas e a ROM, baixa e compila o 3dsrecomp
na versão com que este projeto foi testado, recompila o código do jogo com
as mudanças deste projeto, compila o programa e copia os cheats para a pasta
de cheats do Zakuro (acrescentando os que faltam a um arquivo de cheats que
já exista, sem mexer no resto). A recompilação leva **20 minutos ou mais**; use
`--low-priority` para continuar usando o PC enquanto isso. Rodar de novo
depois só refaz o que mudou. `cargo build-game --help` mostra as opções.

No fim, o programa fica em `target\release\smt4.exe`.

No macOS é igual, com caminhos Unix, e o programa fica em
`target/release/smt4`:

```
cargo build-game ~/Games/"Shin Megami Tensei IV (USA).3ds"
```

Com `--bundle`, ele também monta o `target/release/SMT4.app`, que abre pelo
Finder com a ROM usada na compilação (ou uma arrastada para ele) e leva o
MoltenVK dentro, sem precisar do Homebrew para rodar. Como o programa, ele
contém o código do jogo: não compartilhe.

## Jogando

```
target\release\smt4.exe "C:\caminho\para\Shin Megami Tensei IV (USA).3ds"
```

Ele lê os arquivos do jogo e tudo menos o código do seu dump, então deixe a
ROM onde está. Saves, save states e configurações ficam na pasta do Zakuro,
`%APPDATA%\zakuro` (no macOS, `~/Library/Application Support/zakuro`).

Configuração recomendada (Escape abre o menu):

1. **Settings**, Layout: **Top screen only**. F10 alterna para a tela de
   baixo e volta, para o mapa e os menus.
2. **Settings**, Top screen shape: **16:9** (ou 21:9 num monitor ultrawide).
3. **Cheats**: ligue **Widescreen 16:9** (o mesmo formato) e vá para outra
   área, para o jogo recalcular a câmera.

## Cheats e melhorias

Escape, depois **Cheats**. Os marcados *smt4-recomp* só funcionam nesta
versão; os *Enhancements* vêm embutidos no Zakuro, no topo da janela; os
outros são códigos Action Replay comuns.

| Cheat | O que faz |
|---|---|
| Widescreen 16:9 / 21:9 *(smt4-recomp)* | Cena 3D mais larga. Precisa do Top screen shape no mesmo formato, e aparece na próxima troca de área. |
| Macca x2 / x3 / x5 *(smt4-recomp)* | Multiplica o Macca de batalhas, lojas e recompensas. A loja mostra o preço normal; o dinheiro recebido é multiplicado. |
| Hero EXP x2 / x3 / x5 *(smt4-recomp)* | Multiplica a EXP do herói nas batalhas. |
| Demon EXP x2 / x3 / x5 *(smt4-recomp)* | Multiplica a EXP dos demônios (grupo e estoque). |
| App Points x2 / x3 / x5 *(smt4-recomp)* | Multiplica os App Points. |
| Easy Recruit *(smt4-recomp)* | Toda negociação dá certo: as respostas funcionam, recusar ou não ter o que o demônio pede ainda funciona, e ele não foge nem te engana. Ele ainda pede coisas. |
| 60 FPS *(Enhancements)* | A lógica do jogo segue o frame rate, então tudo fica **duas vezes mais rápido**. Desligar **não** volta a velocidade normal: use o 30 FPS para isso. |
| 30 FPS *(Enhancements)* | O padrão do jogo; desfaz o 60 FPS. |
| Max Macca, 9999 App points, All items, Hero Max Exp, Hero HP/MP, Hero Stats | Códigos da comunidade. |

Ligue só um de cada multiplicador. Desligar um multiplicador ou o widescreen
volta ao normal sozinho.

Como funcionam (em inglês): [MULTIPLIERS.md](docs/MULTIPLIERS.md),
[WIDESCREEN.md](docs/WIDESCREEN.md), [RECRUIT.md](docs/RECRUIT.md).

## Controles

Padrões do teclado, todos alteráveis em Settings. Controles também funcionam.

| 3DS | Tecla | | Zakuro | Tecla |
|---|---|---|---|---|
| A / B | X / Z | | Menu | Escape |
| X / Y | S / A | | Pausar | F1 |
| L / R | Q / W | | Salvar / carregar state | F5 / F8 |
| ZL / ZR | 1 / 2 | | Próximo layout | F9 |
| Start / Select | Enter / Backspace | | Tela de cima / de baixo | F10 |
| Direcional | Setas | | Tela cheia | F11 |
| Circle Pad | I J K L | | Acelerar | segurar Tab |
| C-Stick | T F G H | | Tela de toque | mouse |

Num teclado de Mac, as teclas F precisam do Fn, a não ser que estejam como
teclas de função padrão em Ajustes do Sistema, Teclado.

## Extraindo assets

O `tools/extract` tira os arquivos do jogo da sua ROM e converte: texturas
para PNG, modelos para CGFX puro (para SPICA, Ohana3DS ou Noesis), textos
para UTF-8, música, efeitos e vozes para WAV, cutscenes para MP4.

```
cd tools\extract
cargo build --release
target\release\smt4-extract "C:\caminho\para\Shin Megami Tensei IV (USA).3ds" C:\smt4-assets
```

Os formatos, o que é convertido e o que não é (em inglês):
[ASSETS.md](docs/ASSETS.md). Como na compilação, o que ele gera é do jogo: não
compartilhe.

Para trocar uma imagem do jogo por uma sua (um retrato, um ícone, um fundo),
na mesma resolução ou maior:

```
cargo fit-texture <assets>\textures\tex\stex\stbustup\st_bu_pc001.png minha_imagem.png
```

Ele encaixa a sua imagem onde fica o personagem da original e instala como
texture pack; reabra o jogo para ver. Passo a passo, e o que fazer quando a
imagem não muda (em inglês): [MODDING.md](docs/MODDING.md).

## Problemas comuns

- **"no 64-bit gcc found"**: só há um MinGW antigo de 32 bits instalado (um
  que venha primeiro no PATH é pulado sozinho). Instale o MinGW-w64, por
  exemplo `winget install BrechtSanders.WinLibs.POSIX.UCRT`, ou defina `CC` e
  `AR` com os caminhos completos de um `gcc.exe` e um `ar.exe` de 64 bits.
- **Avisos `.drectve ... unrecognized` e depois um erro de link**: o gcc que
  vem com o Strawberry Perl tem um linker antigo demais para o Rust atual. O
  build pula ele sozinho quando há outro gcc de 64 bits instalado; instale um
  com `winget install BrechtSanders.WinLibs.POSIX.UCRT`.
- **"Rust uses the MSVC toolchain"**, ou erros sobre `link.exe`: rode
  `rustup default stable-x86_64-pc-windows-gnu`.
- **"is title ..., this project is for Shin Megami Tensei IV (USA)"**: só a
  versão americana é suportada; o código muda entre regiões.
- **O jogo está duas vezes mais rápido**: o 60 FPS foi ligado em algum
  momento. Ligue o **30 FPS** (Enhancements) e salve um state de novo.
- **A imagem está comprimida ou esticada**: o cheat de Widescreen e o Top
  screen shape precisam estar no mesmo formato, ou os dois desligados.
- **macOS: "Could not start Vulkan"**: o MoltenVK não está instalado, rode
  `brew install molten-vk`. Um MoltenVK em outro lugar pode ser indicado com
  `ZAKURO_VULKAN_LIB=/caminho/para/libMoltenVK.dylib`.
- **macOS: uma janela pedindo para instalar as ferramentas de
  desenvolvedor**: a compilação procurou o clang ou o git; instale por ela, ou
  com `xcode-select --install`.

## Como funciona

| Parte | Onde | O que faz |
|---|---|---|
| Launcher | `launcher/` | Uma janela em volta do `tools/build`: acha a ROM, confere as ferramentas, compila, abre o jogo |
| Builder | `tools/build` (`cargo build-game`) | Roda todos os passos abaixo, da ROM ao programa |
| Recompilador | [3dsrecomp](https://github.com/fearkov/3dsrecomp) | Encontra as funções do jogo e as escreve em C, em `code/` (fora do repositório) |
| Overrides | `overrides/` | Funções escritas à mão que substituem as geradas: multiplicadores de Macca e App Points, widescreen |
| Patches | `tools/build/src/patch.rs` | Altera o C gerado onde um override não alcança (EXP do herói e dos demônios, a projeção) |
| Programa | `Cargo.toml`, `build.rs`, `src/main.rs` | Junta o código compilado com o Zakuro |
| Runtime | [Zakuro](https://github.com/fearkov/zakuro) | Kernel, serviços, GPU e som do 3DS |
| Mods de imagem | `tools/fit-texture` (`cargo fit-texture`) | Cria e instala uma imagem de texture pack, veja [MODDING.md](docs/MODDING.md) |
| Extrator de assets | `tools/extract/` | Tira os arquivos do jogo da ROM e converte, veja [ASSETS.md](docs/ASSETS.md) |
| Cheats | `cheats/00040000000E5C00.txt` | Códigos do menu de cheats; os *smt4-recomp* escrevem um byte de controle que os overrides leem |

Isto é recompilação estática, não decompilação: não existe código-fonte
legível do jogo aqui nem em nenhuma parte da compilação.

Mais (em inglês): [progresso e problemas conhecidos](docs/PROGRESS.md),
[melhorias e roadmap](docs/ENHANCEMENTS.md).

## Créditos

- **fearkov**: [3dsrecomp](https://github.com/fearkov/3dsrecomp) e
  [Zakuro](https://github.com/fearkov/zakuro), que fazem o trabalho pesado.
- **lucky**: os códigos de 60 FPS e 30 FPS, de
  [Reshiban/60FPS-AR-CHEATS-3DS](https://github.com/Reshiban/60FPS-AR-CHEATS-3DS).
- **JourneyOver** e os autores originais dos códigos: os cheats da
  comunidade, de [JourneyOver/CTRPF-AR-CHEAT-CODES](https://github.com/JourneyOver/CTRPF-AR-CHEAT-CODES).
- *Shin Megami Tensei IV* é da Atlus.

## Licença

[MIT](LICENSE), só para os arquivos deste repositório. Não cobre o jogo nem
nada compilado a partir dele.
