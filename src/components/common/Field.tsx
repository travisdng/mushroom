import { useId } from "react";
import type { InputHTMLAttributes, Ref, TextareaHTMLAttributes } from "react";

type FieldProps = InputHTMLAttributes<HTMLInputElement> & {
  label?: string;
  /** Inline validation message, shown beneath the field. */
  error?: string;
};

export function Field({ label, error, className, id, ...rest }: FieldProps) {
  const generatedId = useId();
  const fieldId = id ?? generatedId;
  const classes = ["field"];
  if (className) classes.push(className);

  const input = (
    <input
      id={fieldId}
      className={classes.join(" ")}
      aria-invalid={error ? true : undefined}
      {...rest}
    />
  );

  if (!label && !error) return input;

  return (
    <>
      <div className="field-row">
        {label ? <label htmlFor={fieldId}>{label}</label> : null}
        {input}
      </div>
      {error ? <div className="field-error">{error}</div> : null}
    </>
  );
}

type TextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & {
  /** React 19 passes `ref` as an ordinary prop; declare it so callers can focus. */
  ref?: Ref<HTMLTextAreaElement>;
};

export function TextArea({ className, ref, ...rest }: TextAreaProps) {
  const classes = ["field"];
  if (className) classes.push(className);
  return <textarea ref={ref} className={classes.join(" ")} {...rest} />;
}
