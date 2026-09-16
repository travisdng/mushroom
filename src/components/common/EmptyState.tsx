import type { ReactNode } from "react";
import { Icon } from "./Icon";

type EmptyStateProps = {
  text: string;
  /** Optional action, e.g. "Create your first note". */
  action?: ReactNode;
};

export function EmptyState({ text, action }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <Icon name="mushroom" size={32} />
      <div>{text}</div>
      {action}
    </div>
  );
}
