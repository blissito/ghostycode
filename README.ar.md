<!-- source: README.md sha256:a56bca473dbd -->
# Ghosty

Ghosty وكيل مفتوح المصدر للبرمجة عبر الطرفية، مبني بلغة Rust ويتطور علنًا بالتعاون مع الأشخاص الذين يستخدمونه.

![Ghosty يعمل في طرفية](assets/screenshot.webp)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [Català](README.ca.md)

[![CI](https://github.com/blissito/ghostycode/actions/workflows/ci.yml/badge.svg)](https://github.com/blissito/ghostycode/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ghosty-cli?label=crates.io)](https://crates.io/crates/ghosty-cli)
[![npm](https://img.shields.io/npm/v/ghosty?label=npm)](https://www.npmjs.com/package/ghosty)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

## التثبيت

```bash
npm install -g ghosty
ghosty
```

يساعدك Ghosty عند التشغيل الأول على الاتصال بموفّر أو البقاء دون اتصال. ويدعم أيضًا Cargo وDocker وNix وScoop والأرشيفات المبنية مسبقًا وAndroid/Termux ومرآة CNB. راجع [دليل التثبيت](docs/INSTALL.md).

يمكن تفعيل الإكمال بمفتاح Tab بأمر واحد لكل واجهة أوامر — `ghosty completion bash|zsh|fish|powershell|elvish`. راجع [إكمال واجهة الأوامر](docs/INSTALL.md#8-shell-completions).

## الاستخدام

تحدث إلى Ghosty كما تتحدث إلى زميل في فريقك:

```text
Fix the failing tests and explain what changed.
```

أو شغّل مهمة من دون فتح واجهة TUI:

```bash
ghosty exec "fix the failing tests and explain what changed"
```

يستطيع Ghosty قراءة مستودعك وتعديل الملفات وتشغيل الأوامر وفحص النتائج ومواصلة العمل نحو هدف. وأنت من يقرر مقدار الوصول الذي تمنحه له.

## لماذا Ghosty

- **استخدم النموذج الذي تريده.** اتصل بموفّرين مستضافين أو بنماذج محلية عبر Ollama أو vLLM أو SGLang. بدّل الموفّر والنموذج باستخدام `/model`.
- **ابقَ مسيطرًا.** وضع Plan للقراءة فقط. تجعل أوضاع Ask وAuto-Review وFull Access سلوك الموافقة واضحًا. يتراجع `/undo` عن الجولة الأخيرة، ويعيد `/restore` مساحة العمل إلى لقطة سابقة.
- **حافظ على تنظيم الأعمال الطويلة.** احفظ الجلسات، وحدد `/goal` دائمًا، وراجع مسارات العمل قبل تشغيلها، ونسّق بين الوكلاء من دون تحويل تعليماتهم الداخلية إلى جزء من محادثتك.
- **وسّع الوكيل الذي لديك بالفعل.** صِل خوادم MCP والمهارات، واضبط الخطافات، واحتفظ بأدوار الوكلاء كملفات مقروءة في مشروعك أو إعداداتك الشخصية.

شغّل `/help` في واجهة TUI لعرض الأوامر واختصارات لوحة المفاتيح.

## الأمان

يعمل Ghosty على جهازك بصلاحيات الوصول التي تمنحها له. تحد أوضاع الموافقة وقواعد المستودع مما يمكن للوكيل فعله؛ ويضيف عزل نظام التشغيل الاختياري حدًا أقوى للتنفيذ حيثما كان مدعومًا. تظل أسعار النماذج غير المعروفة مسجلة على أنها غير معروفة بدلًا من الإبلاغ عنها كمجانية.

اقرأ [ترتيب التفويض](docs/AUTHORIZATION_ORDER.md) لمعرفة التسلسل الدقيق للسياسات، و[الإعدادات](docs/CONFIGURATION.md) لمعرفة الضبط المحلي.

## الوثائق

- [الموفّرون والنماذج المحلية](docs/PROVIDERS.md)
- [فرق الوكلاء](docs/FLEET.md)
- [MCP](docs/MCP.md) و[الخطافات](docs/HOOKS.md) و[الإعدادات](docs/CONFIGURATION.md)
- [عميل الويب المحلي](docs/WEB.md)
- [جميع الوثائق](docs)

## انضم إلى المجتمع

يتحسن Ghosty عندما يستخدمه الناس ويبلغون عما لا يبدو صحيحًا ويساعدون في إصلاحه. إذا كان أحد الموفّرين مفقودًا، أو كان مسار العمل مربكًا، أو كانت واجهة الطرفية تعيقك، [فافتح issue](https://github.com/blissito/ghostycode/issues). وإذا كنت تعرف كيفية تحسينه، [فافتح pull request](CONTRIBUTING.md). نرحب بالمساهمات الأولى، ويظل كل مساهم منسوبًا إلى العمل الذي يُدمج في المشروع.

انضم إلى [Discord](https://discord.gg/37gfS3ksug)، أو أضف Hunter على WeChat (`hunterbown`) واطلب الانضمام إلى مجموعة Whale Brothers.

## تاريخ المشروع

بدأ Ghosty باسم `deepseek-tui`، ولا يزال يحافظ على التوافق مع إعداداته وجلساته. وهو الآن محايد تجاه الموفّرين ويُصان بصورة مستقلة ولا ينتمي إلى أي موفّر نماذج.

شكرًا لكل مساهم ولمجتمعات المصادر المفتوحة التي ساعدت المشروع على النمو. راجع [سجل المساهمين](docs/CONTRIBUTORS.md).

## الترخيص

[MIT](LICENSE). الأجزاء المقتبسة والمعدّلة من مشاريع أخرى مفتوحة المصدر مسجّلة في [إشعارات الجهات الخارجية](docs/THIRD_PARTY_NOTICES.md).
