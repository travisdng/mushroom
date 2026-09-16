import type { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  /** Toolbar buttons are flat until hovered; dialog buttons are always raised. */
  variant?: "default" | "toolbar";
  /** Holds the pressed bevel, for toggles like the preview switch. */
  pressed?: boolean;
};

export function Button({
  children,
  variant = "default",
  pressed,
  className,
  type = "button",
  ...rest
}: ButtonProps) {
  const classes = ["btn"];
  if (variant === "toolbar") classes.push("btn--toolbar");
  if (className) classes.push(className);

  return (
    <button
      type={type}
      className={classes.join(" ")}
      data-pressed={pressed ? "true" : undefined}
      aria-pressed={pressed === undefined ? undefined : pressed}
      {...rest}
    >
      {children}
    </button>
  );
}
