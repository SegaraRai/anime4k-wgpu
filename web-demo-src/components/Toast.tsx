export function Toast({
  message,
  isVisible,
  class: className = "alert-info",
  position = "top",
  align = "center",
}: {
  readonly message: string | null;
  readonly isVisible: boolean;
  readonly class?: string;
  readonly position?: "top" | "bottom";
  readonly align?: "start" | "center" | "end";
}) {
  if (!message) {
    return null;
  }

  const positionClass = position === "top" ? "toast-top" : "toast-bottom";

  let alignClass: string;
  if (align === "start") {
    alignClass = "toast-start";
  } else if (align === "end") {
    alignClass = "toast-end";
  } else {
    alignClass = "toast-center";
  }

  const hiddenTransform =
    position === "top"
      ? "-translate-y-4 opacity-0 scale-95"
      : "translate-y-4 opacity-0 scale-95";

  return (
    <div class={`toast ${positionClass} ${alignClass} z-50`}>
      <div
        class={`alert ${className} transform transition-all duration-300 ease-out ${
          isVisible ? "translate-y-0 opacity-100 scale-100" : hiddenTransform
        }`}
      >
        <span class="whitespace-pre-line">{message}</span>
      </div>
    </div>
  );
}
