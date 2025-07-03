import { useCallback, useEffect, useRef, useState } from "preact/hooks";

export interface DragOptions {
  readonly onUpdate?: (value: number, lastValue: number | null) => void;
  readonly onEnd?: (value: number | null, lastValue: number | null) => void;
  readonly clamp?: boolean;
}

export interface DragController {
  handleMouseDown: (
    event: MouseEvent,
    container: HTMLElement,
    axis: "x" | "y"
  ) => boolean;
  handleTouchStart: (
    event: TouchEvent,
    container: HTMLElement,
    axis: "x" | "y"
  ) => boolean;
}

interface DraggingContext {
  type: "mouse" | "touch";
  container: HTMLElement;
  containerRect: DOMRect;
  axis: "x" | "y";
  clamp: boolean;
}

export function willHandleMouseDown(event: MouseEvent): boolean {
  return event.button === 0;
}

export function willHandleTouchStart(event: TouchEvent): boolean {
  return event.touches.length === 1;
}

function willHandleMouseUp(event: MouseEvent): boolean {
  // In case of double-clicks, `event.buttons` may still contain the left mouse button.
  // Therefore we have to treat the left mouse button as "not pressed" if
  // - `event.button` is 0 (left button),
  // - or `event.buttons` does not contain the left button (i.e., `event.buttons & 1` is 0).
  return event.button === 0 || (event.buttons & 1) === 0;
}

function willHandleTouchEnd(event: TouchEvent): boolean {
  return event.touches.length === 0;
}

export function useDrag(options: DragOptions): DragController {
  const { onUpdate, onEnd, clamp = false } = options;

  const [activeDrag, setActiveDrag] = useState<DraggingContext | null>(null);
  const lastValueRef = useRef<number | null>(null);

  const calcValue = useCallback(
    (
      { axis, containerRect }: Pick<DraggingContext, "axis" | "containerRect">,
      x: number,
      y: number
    ) => {
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
              onUpdate?.(value, lastValueRef.current);
              lastValueRef.current = value;
            },
            { signal }
          );
        }
        document.addEventListener(
          "mouseup",
          (event) => {
            if (!willHandleMouseUp(event)) {
              // If the left mouse button is still pressed, do not end the drag.
              return;
            }

            controller.abort();

            const { clientX, clientY } = event;
            const value = calcValue(activeDrag, clientX, clientY);
            onEnd?.(value, lastValueRef.current);
            lastValueRef.current = null;
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
              onUpdate?.(value, lastValueRef.current);
              lastValueRef.current = value;
            },
            { signal }
          );
        }
        document.addEventListener(
          "touchend",
          (event) => {
            if (!willHandleTouchEnd(event)) {
              // If there are still touches, do not end the drag.
              return;
            }

            controller.abort();

            const touch = event.changedTouches[0];
            if (!touch) {
              onEnd?.(null, lastValueRef.current);
              lastValueRef.current = null;
              return;
            }

            const value = calcValue(activeDrag, touch.clientX, touch.clientY);
            onEnd?.(value, lastValueRef.current);
            lastValueRef.current = null;
          },
          { signal }
        );
        break;
    }

    return () => {
      if (!signal.aborted) {
        onEnd?.(null, lastValueRef.current);
        controller.abort();
      }
      lastValueRef.current = null;
    };
  }, [activeDrag, calcValue, onUpdate, onEnd]);

  const startDrag = useCallback(
    (
      type: "mouse" | "touch",
      container: HTMLElement,
      axis: "x" | "y",
      initialCoords: MouseEvent | Touch
    ): void => {
      const containerRect = container.getBoundingClientRect();
      const initialValue = calcValue(
        { axis, containerRect },
        initialCoords.clientX,
        initialCoords.clientY
      );
      onUpdate?.(initialValue, null);
      lastValueRef.current = initialValue;

      const newActiveDrag: DraggingContext = {
        type,
        container,
        containerRect,
        axis,
        clamp,
      };
      setActiveDrag(newActiveDrag);
    },
    [clamp, calcValue, onUpdate]
  );

  const handleMouseDown = useCallback(
    (event: MouseEvent, container: HTMLElement, axis: "x" | "y"): boolean => {
      if (!willHandleMouseDown(event)) {
        return false;
      }

      startDrag("mouse", container, axis, event);

      const controller = new AbortController();
      const { signal } = controller;

      document.addEventListener(
        "mouseup",
        () => {
          if (!willHandleMouseUp(event)) {
            // If the left mouse button is still pressed, do not end the drag.
            return;
          }

          controller.abort();

          setActiveDrag(null);
        },
        { signal }
      );

      return true;
    },
    [startDrag]
  );

  const handleTouchStart = useCallback(
    (event: TouchEvent, container: HTMLElement, axis: "x" | "y"): boolean => {
      if (!willHandleTouchStart(event)) {
        return false;
      }

      startDrag("touch", container, axis, event.touches[0]);

      const controller = new AbortController();
      const { signal } = controller;

      document.addEventListener(
        "touchend",
        () => {
          if (!willHandleTouchEnd(event)) {
            // If there are still touches, do not end the drag.
            return;
          }

          controller.abort();

          setActiveDrag(null);
        },
        { signal }
      );

      return true;
    },
    [startDrag]
  );

  return {
    handleMouseDown,
    handleTouchStart,
  };
}
