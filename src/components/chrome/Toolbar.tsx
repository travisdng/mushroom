import { Button } from "../common/Button";
import { Icon } from "../common/Icon";
import type { IconName } from "../common/Icon";

export type ToolbarAction = {
  label: string;
  icon: IconName;
  onClick?: () => void;
  /** Absent handler means the feature does not exist yet — render disabled (R2.4). */
  disabled?: boolean;
  pressed?: boolean;
  separatorBefore?: boolean;
};

/**
 * Icon-above-label buttons, as they were. A button with no handler renders
 * disabled rather than being hidden, so the toolbar shows the shape of the
 * finished application from the first release.
 */
export function Toolbar({ actions }: { actions: ToolbarAction[] }) {
  return (
    <div className="toolbar" role="toolbar" aria-label="Main toolbar">
      {actions.map((action) => (
        <span key={action.label} style={{ display: "contents" }}>
          {action.separatorBefore ? (
            <span className="toolbar__separator" aria-hidden="true" />
          ) : null}
          <Button
            variant="toolbar"
            onClick={action.onClick}
            disabled={action.disabled ?? !action.onClick}
            pressed={action.pressed}
            title={action.label}
          >
            <Icon name={action.icon} />
            {action.label}
          </Button>
        </span>
      ))}
    </div>
  );
}
