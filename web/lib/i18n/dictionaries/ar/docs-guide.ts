import type { DocsGuideDict } from "../types";

/**
 * Arabic dictionary for the docs "Getting started" page. Arabic script
 * needs roomier leading than the Latin reference — loose, short of the
 * CJK treatment.
 */
export const docsGuide: DocsGuideDict = {
  metaTitle: "البداية · توثيق Ghosty",
  metaDescription:
    "المسار الكامل من التثبيت إلى أسطولك المثالي: التثبيت، وأول جلسة بلا مفاتيح، وربط مزوّد، وإعداد الأسطول.",
  bodyClassName: "text-ink-soft leading-loose",
  overviewTitle: "البداية",
  overviewLead:
    "أربع خطوات من أمر تثبيت واحد إلى أسطول جاهز لعملك.",
  sessionTitle: "شاهد جلسة حقيقية",
  sessionLead:
    "سيوضع هنا تسجيل لجلسة حقيقية. لا يوجد تسجيل بعد، لذا لا يُعرض شيء.",
  nextTitle: "إلى أين بعد ذلك",
  sourceNote:
    "المستندات المصدر: docs/GUIDE.md، docs/KEYBINDINGS.md · نصوص الخطوات في web/lib/content/getting-started.ts؛ حدّث docs-map.ts عند أي تغيير.",
};
