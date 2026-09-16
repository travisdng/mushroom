import { useCallback, useEffect, useRef, useState } from "react";

export type MenuItemDef =
  | { type: "separator" }
  | {
      type: "item";
      label: string;
      /** Letter to underline once Alt has been pressed. Must appear in label. */
      mnemonic?: string;
      accel?: string;
      checked?: boolean;
      /** No handler means the feature does not exist yet — render disabled (R3.6). */
      onSelect?: () => void;
      disabled?: boolean;
    };

export type MenuDef = {
  title: string;
  mnemonic: string;
  items: MenuItemDef[];
};

/** Underline the mnemonic letter inside a label. */
function Label({ text, mnemonic }: { text: string; mnemonic?: string }) {
  if (!mnemonic) return <>{text}</>;
  const at = text.toLowerCase().indexOf(mnemonic.toLowerCase());
  if (at < 0) return <>{text}</>;
  return (
    <>
      {text.slice(0, at)}
      <span className="mnemonic">{text.slice(at, at + 1)}</span>
      {text.slice(at + 1)}
    </>
  );
}

function isSelectable(item: MenuItemDef): boolean {
  return item.type === "item" && !item.disabled && Boolean(item.onSelect);
}

export function MenuBar({ menus }: { menus: MenuDef[] }) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  const [activeItem, setActiveItem] = useState<number | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => {
    setOpenIndex(null);
    setActiveItem(null);
  }, []);

  const openMenu = useCallback((index: number) => {
    setOpenIndex(index);
    setActiveItem(null);
  }, []);

  // Click away and window blur both close the menu (R3.4).
  useEffect(() => {
    if (openIndex === null) return;

    const onPointerDown = (e: MouseEvent) => {
      if (!barRef.current?.contains(e.target as Node)) close();
    };
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("blur", close);
    };
  }, [openIndex, close]);

  // Alt reveals mnemonics and focuses the bar; Alt+letter opens that menu (R3.3).
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Alt" && !e.repeat) {
        document.querySelector(".app")?.setAttribute("data-mnemonics", "true");
        return;
      }
      if (!e.altKey) return;
      const index = menus.findIndex(
        (m) => m.mnemonic.toLowerCase() === e.key.toLowerCase(),
      );
      if (index >= 0) {
        e.preventDefault();
        document.querySelector(".app")?.setAttribute("data-mnemonics", "true");
        openMenu(index);
        barRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [menus, openMenu]);

  const moveItem = (menu: MenuDef, delta: number) => {
    const count = menu.items.length;
    let next = activeItem ?? (delta > 0 ? -1 : count);
    for (let step = 0; step < count; step += 1) {
      next = (next + delta + count) % count;
      const item = menu.items[next];
      if (item && isSelectable(item)) {
        setActiveItem(next);
        return;
      }
    }
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (openIndex === null) return;
    const menu = menus[openIndex];
    if (!menu) return;

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        close();
        break;
      case "ArrowRight":
        e.preventDefault();
        openMenu((openIndex + 1) % menus.length);
        break;
      case "ArrowLeft":
        e.preventDefault();
        openMenu((openIndex - 1 + menus.length) % menus.length);
        break;
      case "ArrowDown":
        e.preventDefault();
        moveItem(menu, 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveItem(menu, -1);
        break;
      case "Enter":
      case " ": {
        e.preventDefault();
        const item = activeItem === null ? undefined : menu.items[activeItem];
        if (item && item.type === "item" && item.onSelect) {
          close();
          item.onSelect();
        }
        break;
      }
      default:
        break;
    }
  };

  return (
    <div
      ref={barRef}
      className="menubar"
      role="menubar"
      tabIndex={-1}
      onKeyDown={onKeyDown}
    >
      {menus.map((menu, i) => (
        <div key={menu.title} style={{ position: "relative", display: "flex" }}>
          <button
            type="button"
            className="menubar__title"
            role="menuitem"
            aria-haspopup="menu"
            aria-expanded={openIndex === i}
            onClick={() => (openIndex === i ? close() : openMenu(i))}
            // While a menu is open, hovering another title switches to it (R3.2).
            onMouseEnter={() => openIndex !== null && openMenu(i)}
          >
            <Label text={menu.title} mnemonic={menu.mnemonic} />
          </button>

          {openIndex === i ? (
            <div className="menu" role="menu" aria-label={menu.title}>
              {menu.items.map((item, j) =>
                item.type === "separator" ? (
                  <div
                    key={`sep-${j}`}
                    className="menu__separator"
                    role="separator"
                  />
                ) : (
                  <button
                    key={item.label}
                    type="button"
                    className="menuitem"
                    role="menuitem"
                    data-active={activeItem === j ? "true" : undefined}
                    disabled={item.disabled ?? !item.onSelect}
                    onMouseEnter={() => setActiveItem(j)}
                    onClick={() => {
                      close();
                      item.onSelect?.();
                    }}
                    style={{ position: "relative" }}
                  >
                    {item.checked ? (
                      <span className="menuitem__check" aria-hidden="true">
                        ✓
                      </span>
                    ) : null}
                    <span className="menuitem__label">
                      <Label text={item.label} mnemonic={item.mnemonic} />
                    </span>
                    {item.accel ? (
                      <span className="menuitem__accel">{item.accel}</span>
                    ) : null}
                  </button>
                ),
              )}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
