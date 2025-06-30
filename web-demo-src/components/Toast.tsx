import clsx from "clsx";

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
    <div class={clsx("toast", positionClass, alignClass, "z-50")}>
      <div
        class={clsx(
          "alert",
          "transform transition-all duration-300 ease-out",
          className,
          isVisible ? "translate-y-0 opacity-100 scale-100" : hiddenTransform
        )}
        role="alert"
        aria-live="polite"
      >
        <span class="whitespace-pre-line">{message}</span>
      </div>
    </div>
  );
}
