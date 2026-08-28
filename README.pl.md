<!-- source: README.md sha256:a56bca473dbd -->
# Ghosty

Ghosty to agent programistyczny o otwartym kodzie źródłowym do terminala, napisany w Rust i rozwijany publicznie wspólnie z osobami, które go używają.

![Ghosty działający w terminalu](assets/screenshot.webp)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [العربية](README.ar.md) · [Català](README.ca.md)

[![CI](https://github.com/blissito/ghostycode/actions/workflows/ci.yml/badge.svg)](https://github.com/blissito/ghostycode/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ghosty-cli?label=crates.io)](https://crates.io/crates/ghosty-cli)
[![npm](https://img.shields.io/npm/v/ghosty?label=npm)](https://www.npmjs.com/package/ghosty)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

## Instalacja

```bash
npm install -g ghosty
ghosty
```

Przy pierwszym uruchomieniu Ghosty pomaga połączyć się z dostawcą lub pozostać w trybie offline. Obsługuje też Cargo, Docker, Nix, Scoop, gotowe archiwa, Android/Termux oraz serwer lustrzany CNB. Zobacz [instrukcję instalacji](docs/INSTALL.md).

Uzupełnianie klawiszem Tab można włączyć jednym poleceniem dla każdej powłoki — `ghosty completion bash|zsh|fish|powershell|elvish`. Zobacz [uzupełnianie powłoki](docs/INSTALL.md#8-shell-completions).

## Użycie

Rozmawiaj z Ghosty tak, jak z osobą ze swojego zespołu:

```text
Fix the failing tests and explain what changed.
```

Możesz też uruchomić zadanie bez otwierania TUI:

```bash
ghosty exec "fix the failing tests and explain what changed"
```

Ghosty może czytać Twoje repozytorium, edytować pliki, wykonywać polecenia, sprawdzać wyniki i kontynuować pracę nad celem. Ty decydujesz, jaki poziom dostępu mu przyznasz.

## Dlaczego Ghosty

- **Używaj wybranego modelu.** Połącz się z hostowanymi dostawcami lub lokalnymi modelami przez Ollama, vLLM albo SGLang. Dostawcę i model zmienisz za pomocą `/model`.
- **Zachowaj kontrolę.** Tryb Plan jest tylko do odczytu. Ask, Auto-Review i Full Access jasno pokazują sposób zatwierdzania działań. `/undo` cofa ostatnią turę, a `/restore` przywraca przestrzeń roboczą do wcześniejszej migawki.
- **Utrzymuj porządek w długich zadaniach.** Zapisuj sesje, ustawiaj trwały `/goal`, sprawdzaj przepływy pracy przed uruchomieniem i koordynuj agentów bez umieszczania ich wewnętrznych instrukcji w zapisie Twojej rozmowy.
- **Rozszerzaj agenta, którego już masz.** Podłączaj serwery MCP i umiejętności, konfiguruj hooki oraz przechowuj role agentów jako czytelne pliki w projekcie lub ustawieniach osobistych.

Uruchom `/help` w TUI, aby zobaczyć polecenia i skróty klawiaturowe.

## Bezpieczeństwo

Ghosty działa na Twoim komputerze z dostępem, który mu przyznasz. Tryby zatwierdzania i reguły repozytorium ograniczają działania agenta; opcjonalny sandbox systemu operacyjnego zapewnia mocniejszą granicę wykonywania tam, gdzie jest obsługiwany. Nieznane ceny modeli pozostają oznaczone jako nieznane, zamiast być przedstawiane jako bezpłatne.

Przeczytaj o [kolejności autoryzacji](docs/AUTHORIZATION_ORDER.md), aby poznać dokładną hierarchię zasad, oraz o [konfiguracji](docs/CONFIGURATION.md), aby poznać ustawienia lokalne.

## Dokumentacja

- [Dostawcy i modele lokalne](docs/PROVIDERS.md)
- [Zespoły agentów](docs/FLEET.md)
- [MCP](docs/MCP.md), [hooki](docs/HOOKS.md) i [konfiguracja](docs/CONFIGURATION.md)
- [Lokalny klient webowy](docs/WEB.md)
- [Cała dokumentacja](docs)

## Dołącz do społeczności

Ghosty staje się lepszy, gdy ludzie go używają, zgłaszają niedogodności i pomagają je naprawiać. Jeśli brakuje dostawcy, przepływ pracy jest niewygodny albo interfejs terminala przeszkadza Ci w pracy, [otwórz issue](https://github.com/blissito/ghostycode/issues). Jeśli wiesz, jak coś ulepszyć, [otwórz pull request](CONTRIBUTING.md). Pierwsze wkłady są mile widziane, a autorzy zachowują uznanie za pracę przyjętą do projektu.

Dołącz do [Discorda](https://discord.gg/37gfS3ksug) albo dodaj Huntera na WeChat (`hunterbown`) i poproś o dołączenie do grupy Whale Brothers.

## Historia projektu

Ghosty rozpoczął się jako `deepseek-tui` i nadal zachowuje zgodność z jego konfiguracją oraz sesjami. Obecnie jest niezależny od dostawców, utrzymywany samodzielnie i nie jest powiązany z żadnym dostawcą modeli.

Dziękujemy wszystkim współtwórcom oraz społecznościom open source, które pomogły projektowi się rozwijać. Zobacz [rejestr współtwórców](docs/CONTRIBUTORS.md).

## Licencja

[MIT](LICENSE). Części zaadaptowane z innych projektów open source są wymienione w [informacjach o komponentach zewnętrznych](docs/THIRD_PARTY_NOTICES.md).
