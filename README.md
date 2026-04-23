# 🦀 OptiRust: PNG Multi-threaded Optimizer

![Version](https://img.shields.io/badge/version-0.1.2-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Docker Size](https://img.shields.io/badge/docker%20image-2.53MB-blueviolet)
![Rust](https://img.shields.io/badge/rust-2024-orange?logo=rust)

O **OptiRust** é uma ferramenta de linha de comando (CLI) desenvolvida em **Rust** focada na otimização em massa de imagens PNG. O projeto foi construído aplicando conceitos avançados de sistemas, como processamento paralelo, serialização rigorosa e desenvolvimento guiado por testes (TDD).

> [!WARNING]
> Os arquivos otimizados sobreescrevem os originais!

## 🚀 Diferenciais Técnicos
- **Processamento Paralelo (Rayon):** Diferente de scripts sequenciais, o OptiRust utiliza um pool de threads para processar múltiplas imagens simultaneamente, escalando a performance de acordo com os núcleos da CPU.

- **Arquitetura Robusta:** Separação clara entre o motor de compressão (`optimizer`), o rastreador de arquivos (`scanner`) e o gerador de métricas (`report`).

- **Segurança de Memória:** Aproveita o sistema de _ownership_ do Rust para garantir que a manipulação de arquivos e buffers de memória seja livre de _data races_.

- **Configuração Flexível:** Suporte a arquivos `optirust.toml` para definição de níveis de compressão e persistência de preferências.

## 🛠️ Tecnologias Utilizadas
- **Engine:** `oxipng` (Motor de compressão de alto desempenho).

- **CLI & UX:** `clap` (Parsing de argumentos) e `indicatif` (Barras de progresso dinâmicas).

- **Paralelismo:** `rayon` (Data parallelism library).

- **Data Handling:** `serde` (JSON/TOML) e `chrono` (Timestamping).

- **Estilização:** `colored` (ANSI terminal colors).

## 🏗️ Arquitetura do Projeto

```
├── src/
│   ├── main.rs                 # Entrada do programa
│   ├── config.rs               # Configuração e Persistência
│   ├── report.rs               # Gerador de Relatórios
│   ├── scanner.rs              # Rastreador de Arquivos
│   └── optimizer.rs            # Motor de Compressão
├── demo/                   
│   ├── assets/                 # Imagens para demonstração
│   ├── index.html              # Página da demonstração
│   └── iniciar_demo.txt        # Altere este arquivo para iniciar o Actions
├── .github/workflows/demo.yml  # Pipeline CI/CD de demonstração
├── docs/                       # Documentação do projeto
├── examples/                   # Scripts criados durante o desenvolvimento 
├── Cargo.toml                  # Configuração do Cargo
├── Cargo.lock                  # Lockfile do Cargo
├── Dockerfile                  # Arquivo Docker
├── optirust_report.json        # Relatório real
├── optirust.toml               # Configuração padrão
├── test_input.png              # Imagem para testes unitários
└── README.md                   # Documentação
```

## 📦 Como Instalar e Rodar
**Pré-requisitos**

- Rust (Cargo) 1.70+

- Ambiente Linux (Homologado em Arch Linux)

**Compilação de Alta Performance**

Para obter o máximo de desempenho do paralelismo, compile sempre em modo release:

```Bash
cargo build --release
```

**Comandos Principais**

Inicializar Configuração:

```Bash
./target/release/optirust init
```

**Executar Otimização com Relatório Visual**

```Bash
./target/release/optirust run ./assets --summary
```

### 📊 Exemplo de Relatório (JSON)
Ao final de cada execução, o sistema gera um `optirust_report.json` detalhado para auditoria:

```JSON
{
  "timestamp": "2026-03-17T14:20:00Z",
  "status": "success",
  "summary": {
    "files_processed": 12,
    "total_original_size_kb": 4500.5,
    "total_optimized_size_kb": 3200.2,
    "space_saved_kb": 1300.3,
    "efficiency_gain_percent": 28.9
  },
  "files": [
    {
      "name": "hero-banner.png",
      "path": "./assets/hero-banner.png",
      "original_kb": 1200.0,
      "optimized_kb": 850.5,
      "saved_kb": 349.5,
      "ratio": "29.1%"
    }
  ]
}
```

## 🐳 Uso via Docker (DevOps Ready)
O OptiRust está disponível como uma imagem Docker ultra-leve (baseada em scratch), ideal para ser integrada em pipelines de CI/CD para otimização automática de assets.

### 1. Rodando a imagem do Docker Hub

```Bash
# Otimizando uma pasta local usando o container
docker run --rm -t -v $(pwd)/assets:/data betoxvt/optirust:latest run /data --summary
```
> [!TIP]
> Use a flag `-t` do `docker run` para ver o relatório colorido!

### 2. Construindo a imagem localmente
Utilizamos um processo de Multi-stage build para garantir que a imagem final contenha apenas o binário estático:

```Bash
docker build -t optirust .
```

### 3. Integração em Pipeline (Exemplo GitHub Actions)
Você pode usar o OptiRust para otimizar imagens antes do deploy:

```YAML
- name: Optimize PNGs
  run: |
    docker run --rm -v ${{ github.workspace }}/assets:/assets \
    betoxvt/optirust:latest run /assets
```
## 🧪 Quick Lab: Teste o Optirust em 30 segundos
Você pode testar o poder de compressão do Optirust diretamente no seu navegador, usando o GitHub Actions como laboratório.
### 1. Preparação
1. Faça um ***Fork*** deste repositório.
2. No seu fork, vá na aba Actions e clique no botão verde para habilitar os workflows (*"I understand my workflows, go ahead and enable them"*).
3. No seu *fork*, vá em **Settings > Pages**. Em **Build and deployment > Source**, altere para **GitHub Actions**.
### 2. O Cenário Inicial
1. Aguarde o primeiro deploy (veja na aba Actions).
2. Acesse a URL gerada (ex: https://seu-usuario.github.io/optirust/).
3. Você verá uma galeria com 20 imagens marcadas como "Aguardando otimização...". Note que os tamanhos exibidos são os originais (baseados no sistema de arquivos).
### 3. Disparando a Otimização
Agora, vamos ver o Optirust em ação no pipeline:
1. No seu repositório, vá até o arquivo `demo/iniciar_demo.txt`.
2. Clique no ícone de lápis para editar e altere qualquer detalhe inofensivo (ex: adicione qualquer palavra).
3. Clique em ***Commit changes...*** para salvar na main.
### 4. O Resultado
1. Vá na aba Actions e acompanhe o workflow "Optirust Demo Lab". Você verá o Docker esmagando as imagens em tempo real.
2. Quando terminar, volte à sua página do GitHub Pages e dê F5.
3. Mágica: A tabela agora mostrará o tamanho "Original" (riscado) e o novo tamanho "Otimizado", calculando a porcentagem exata de economia de espaço.

Os resultados devem ser próximos aos expressos nesta tabela:

| **Categoria**   | **Métrica**                     | **Valor Obtido**       |
| --------------- | ------------------------------- | ---------------------- |
| **Compressão**  | Economia Total de Dados         | 2.901,21 KB (~2,83 MB) |
| **Compressão**  | Ganho Médio de Eficiência       | 12,0%                  |
| **Compressão**  | Melhor Caso Individual          | 66,0% (Arquivo 15.png) |
| **Compressão**  | Pior Caso Individual            | 1,7% (Arquivo 04.png)  |
| **Performance** | Tempo Total do Pipeline (CI/CD) | 26s                    |
| **Performance** | Tempo de Vida do Contêiner      | 9s                     |
| **Performance** | Tempo Líquido de Compressão     | 6s                     |

### 🛠️ Como funciona este teste?
Este repositório utiliza um pipeline de CI/CD que:
- **Não altera seu código:** As imagens originais na pasta `demo/assets` continuam pesadas no Git.
- **Otimização *On-the-fly*:** O GitHub Actions usa a imagem Docker `betoxvt/optirust` para otimizar os assets apenas durante o build.
- **Deploy Transparente:** Apenas as versões leves são enviadas para o servidor de hospedagem.

**Diagrama do fluxo operacional:**
```mermaid
graph TD
    %% Estilos de Alto Contraste Coloridos
    %% Forçamos o texto a ser preto (#000) ou branco (#fff) para contraste.
    classDef default fill:#fff,stroke:#000,stroke-width:1px,color:#000;
    classDef user fill:#ff80ab,stroke:#c51162,stroke-width:2px,color:#000,font-weight:bold;
    classDef git fill:#81d4fa,stroke:#01579b,stroke-width:2px,color:#000;
    classDef actions fill:#ffe082,stroke:#ff6f00,stroke-width:2px,color:#000;
    classDef docker fill:#80cbc4,stroke:#004d40,stroke-width:2px,color:#000;
    classDef pages fill:#81c784,stroke:#1b5e20,stroke-width:2px,color:#000;

    subgraph Ambiente_Usuario [Ambiente do Usuario]
        U1["Faça o Fork do Repositório"]:::user --> U2["Habilite o GitHub Actions"]:::user
        U2 --> U3["Habilite o GitHub Pages"]:::user
        U3 --> U4["Realize um Commit na pasta /demo"]:::user
        U6["Atualize a Página e Compare os Tamanhos"]:::user
    end

    subgraph GitHub_Infra [Infraestrutura do GitHub]
        G1["Repositório Forkado (main)"]:::git -->|Gatilho: Push em /demo| A1
        U4 -->|Push| G1
        
        subgraph GActions [GitHub Actions Runner]
            A1["Inicie o Workflow (demo.yml)"]:::actions --> A2["Execute o optirust (Docker)"]:::actions
            A2 --> A3["Upload do Artefato (/demo)"]:::actions
        end
    end

    subgraph Recursos_Externos [Recursos Externos]
        D1(("DockerHub: optirust")):::docker -.->|Pull da Imagem| A2
    end

    subgraph Publicacao [Publicação]
        A3 -->|Deploy| P1("GitHub Pages Site"):::pages
        P1 -.->|Visualização| U6
    end

    %% Relacionamentos
    U1 -.-> G1
```

## ✅ Funcionalidades Implementadas (v0.1.0)

- [x] **Core Engine em Rust:** Processamento de alta performance utilizando a Edition 2024.
- [x] **Paralelismo com Rayon:** Otimização multi-thread para processamento em massa de diretórios.
- [x] **Interface CLI Intuitiva:** Comandos estruturados via `Clap` com suporte a ajuda detalhada (`--help`).
- [x] **Relatórios Dinâmicos:** Geração automática de `optirust_report.json` e resumo visual colorido no terminal.
- [x] **Containerização Estática:** Imagem Docker ultra-leve baseada em `scratch` (<3MB), garantindo portabilidade total.
- [x] **Cálculo de Eficiência:** Métricas precisas de ganho de espaço por arquivo e total da operação.

## 🗺️ Roadmap de Desenvolvimento

Abaixo estão as funcionalidades planejadas para as próximas iterações do OptiRust:

- [ ] **Modo Silent (`--silent`):** Redução de logs para integração limpa em CI/CD.
- [ ] **Configuração Dinâmica:** Suporte a `--config <path>` e argumentos de linha de comando para sobrescrever o `optirust.toml`.
- [ ] **Suporte Multi-Formato:** Expansão do algoritmo de compressão para:
    - [ ] 🖼️ **Imagens:** WebP, JPEG e SVG.
    - [ ] 📄 **Documentos:** Otimização de camadas de PDF.
- [ ] **Internacionalização (i18n):** Suporte a logs e relatórios em Inglês e Português.
- [ ] **Post-Compile Optimization (PCO):** Utilizar flags do Cargo.toml, seção `[profile.release]` para otimizar o binário final.
