import { useCallback, useEffect, useState } from "preact/hooks";

export interface DragOptions {
  readonly onUpdate?: (value: number) => void;
  readonly onEnd?: (value: number) => void;
  readonly clamp?: boolean;
}

export interface DragController {
  handleMouseDown: (
    container: HTMLElement,
    axis: "x" | "y",
    initialEvent?: MouseEvent
  ) => void;
  handleTouchStart: (
    container: HTMLElement,
    axis: "x" | "y",
    initialEvent?: TouchEvent
  ) => void;
}

interface DraggingContext {
  type: "mouse" | "touch";
  container: HTMLElement;
  containerRect: DOMRect;
  axis: "x" | "y";
  clamp?: boolean;
}

export function useDrag(options: DragOptions): DragController {
  const { onUpdate, onEnd, clamp = false } = options;

  const [activeDrag, setActiveDrag] = useState<DraggingContext | null>(null);

  const calcValue = useCallback(
    ({ axis, containerRect }: DraggingContext, x: number, y: number) => {
      const raw =
        axis === "x"
          ? (x - containerRect.left) / containerRect.width
          : (y - containerRect.top) / containerRect.height;
      return clamp ? Math.max(0, Math.min(raw, 1)) : raw;
    },
    [clamp]
  );

  useEffect(() => {
    if (!activeDrag) {
      return;
    }

    const observer = new ResizeObserver(() => {
      setActiveDrag((prev) => {
        if (prev?.container !== activeDrag.container) {
          return prev;
        }

        const containerRect = activeDrag.container.getBoundingClientRect();
        if (
          containerRect.x === prev.containerRect.x &&
          containerRect.y === prev.containerRect.y &&
          containerRect.width === prev.containerRect.width &&
          containerRect.height === prev.containerRect.height
        ) {
          return prev;
        }

        return {
          ...prev,
          containerRect,
        };
      });
    });
    observer.observe(activeDrag.container, { box: "border-box" });

    return () => {
      observer.disconnect();
    };
  }, [activeDrag?.container]);

  useEffect(() => {
    if (!activeDrag) {
      return;
    }

    const controller = new AbortController();
    const { signal } = controller;

    switch (activeDrag.type) {
      case "mouse":
        if (onUpdate) {
          document.addEventListener(
            "mousemove",
            (event) => {
              const { clientX, clientY } = event;
              const value = calcValue(activeDrag, clientX, clientY);
              onUpdate?.(value);
            },
            { signal }
          );
        }
        document.addEventListener(
          "mouseup",
          (event) => {
            controller.abort();

            const { clientX, clientY } = event;
            const value = calcValue(activeDrag, clientX, clientY);
            onEnd?.(value);
          },
          { signal }
        );
        break;

      case "touch":
        if (onUpdate) {
          document.addEventListener(
            "touchmove",
            (event) => {
              const touch = event.touches[0];
              if (!touch) {
                return;
              }

              const value = calcValue(activeDrag, touch.clientX, touch.clientY);
              onUpdate?.(value);
            },
            { signal }
          );
        }
        document.addEventListener(
          "touchend",
          (event) => {
            controller.abort();

            const touch = event.changedTouches[0];
            if (!touch) {
              return;
            }

            const value = calcValue(activeDrag, touch.clientX, touch.clientY);
            onEnd?.(value);
          },
          { signal }
        );
        break;
    }

    return () => {
      controller.abort();
    };
  }, [activeDrag, calcValue, onUpdate, onEnd]);

  const startDrag = useCallback(
    (
      type: "mouse" | "touch",
      container: HTMLElement,
      axis: "x" | "y",
      initialCoords?: MouseEvent | Touch
    ): void => {
      const containerRect = container.getBoundingClientRect();
      const newActiveDrag: DraggingContext = {
        type,
        container,
        containerRect,
        axis,
        clamp,
      };
      setActiveDrag(newActiveDrag);

      if (initialCoords) {
        const value = calcValue(
          newActiveDrag,
          initialCoords.clientX,
          initialCoords.clientY
        );
        onUpdate?.(value);
      }
    },
    [clamp, calcValue, onUpdate]
  );

  const handleMouseDown = useCallback(
    (
      container: HTMLElement,
      axis: "x" | "y",
      initialEvent?: MouseEvent
    ): void => {
      startDrag("mouse", container, axis, initialEvent);

      document.addEventListener(
        "mouseup",
        () => {
          setActiveDrag(null);
        },
        { once: true }
      );
    },
    [startDrag]
  );

  const handleTouchStart = useCallback(
    (
      container: HTMLElement,
      axis: "x" | "y",
      initialEvent?: TouchEvent
    ): void => {
      startDrag("touch", container, axis, initialEvent?.touches[0]);

      document.addEventListener(
        "touchend",
        () => {
          setActiveDrag(null);
        },
        { once: true }
      );
    },
    [startDrag]
  );

  return {
    handleMouseDown,
    handleTouchStart,
  };
}
