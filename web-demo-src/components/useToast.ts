import { useCallback, useRef, useState } from "preact/hooks";

export interface ToastOptions {
  duration?: number;
  fadeOutDuration?: number;
}

export function useToast(options: ToastOptions = {}) {
  const { duration = 3000, fadeOutDuration = 300 } = options;

  const [message, setMessage] = useState<string | null>(null);
  const [isVisible, setIsVisible] = useState(false);

  const currentTimerId = useRef<ReturnType<typeof setTimeout> | null>(null);

  const showToast = useCallback(
    (newMessage: string, durationOverride?: number) => {
      if (currentTimerId.current != null) {
        clearTimeout(currentTimerId.current);
        currentTimerId.current = null;
      }

      setMessage(newMessage);
      setIsVisible(true);

      // Auto-hide toast after specified duration
      const effectiveDuration = durationOverride ?? duration;
      if (effectiveDuration >= 0 && isFinite(effectiveDuration)) {
        currentTimerId.current = setTimeout(() => {
          setIsVisible(false);

          currentTimerId.current = setTimeout(
            () => setMessage(null),
            fadeOutDuration
          );
        }, effectiveDuration);
      }
    },
    [duration, fadeOutDuration]
  );

  const hideToast = useCallback(() => {
    setIsVisible(false);
    currentTimerId.current = setTimeout(
      () => setMessage(null),
      fadeOutDuration
    );
  }, [fadeOutDuration]);

  return {
    message,
    isVisible,
    showToast,
    hideToast,
  };
}
