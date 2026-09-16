import { useId } from "react";
import type { InputHTMLAttributes, TextareaHTMLAttributes } from "react";

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

type TextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement>;

export function TextArea({ className, ...rest }: TextAreaProps) {
  const classes = ["field"];
  if (className) classes.push(className);
  return <textarea className={classes.join(" ")} {...rest} />;
}
