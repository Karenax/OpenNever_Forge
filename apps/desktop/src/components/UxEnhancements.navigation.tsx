import { Command, Search } from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
} from "react";
import { createPortal } from "react-dom";
import {
  domains,
  filterNavigationCommands,
  type DomainDefinition,
  type NavigationCommand,
  type WorkbenchDomain,
} from "./UxEnhancements.model";

export function DomainNavigation({
  activeDomain,
  onNavigate,
  onOpenPalette,
}: {
  activeDomain: WorkbenchDomain;
  onNavigate: (domain: DomainDefinition) => void;
  onOpenPalette: () => void;
}) {
  return (
    <div className="ux-domain-navigation" role="list" aria-label="Domaines de travail">
      {domains.map((domain, index) => {
        const Icon = domain.icon;
        return (
          <button
            key={domain.id}
            type="button"
            className={domain.id === activeDomain ? "active" : ""}
            aria-pressed={domain.id === activeDomain}
            title={`${domain.label} · Alt+${index + 1}`}
            onClick={() => onNavigate(domain)}
          >
            <Icon size={14} />
            <span>{domain.shortLabel}</span>
          </button>
        );
      })}
      <button
        type="button"
        className="ux-command-nav-button"
        aria-label="Ouvrir la palette de commandes"
        title="Palette de commandes · Ctrl+K"
        onClick={onOpenPalette}
      >
        <Command size={14} />
        <span>Ctrl K</span>
      </button>
    </div>
  );
}

export function CommandPalette({
  open,
  commands,
  onClose,
  onNavigate,
}: {
  open: boolean;
  commands: NavigationCommand[];
  onClose: () => void;
  onNavigate: (command: NavigationCommand) => void;
}) {
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement | null>(null);
  const filtered = useMemo(() => filterNavigationCommands(commands, query), [commands, query]);

  useEffect(() => {
    if (!open) {
      setQuery("");
      return;
    }
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [open]);

  if (!open) return null;
  return createPortal(
    <div className="ux-command-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="ux-command-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Palette de commandes"
        onMouseDown={(event: ReactMouseEvent<HTMLElement>) => event.stopPropagation()}
      >
        <header>
          <Command size={17} />
          <input
            ref={inputRef}
            aria-label="Rechercher une commande"
            placeholder="Ouvrir un atelier…"
            value={query}
            onChange={(event: ChangeEvent<HTMLInputElement>) => setQuery(event.currentTarget.value)}
            onKeyDown={(event: ReactKeyboardEvent<HTMLInputElement>) => {
              if (event.key === "Escape") onClose();
              if (event.key === "Enter" && filtered[0]) onNavigate(filtered[0]);
            }}
          />
          <kbd>Échap</kbd>
        </header>
        <div className="ux-command-results" role="listbox">
          {filtered.map((command) => {
            const Icon = command.icon;
            return (
              <button
                key={command.id}
                type="button"
                role="option"
                aria-selected="false"
                onClick={() => onNavigate(command)}
              >
                <Icon size={16} />
                <span>
                  <strong>{command.label}</strong>
                  <small>{command.description}</small>
                </span>
                <em>{domains.find((domain) => domain.id === command.domain)?.label}</em>
              </button>
            );
          })}
          {filtered.length === 0 && (
            <div className="ux-command-empty">
              <Search size={20} />
              <span>Aucun atelier ne correspond à cette recherche.</span>
            </div>
          )}
        </div>
        <footer>
          <span>Entrée pour ouvrir</span>
          <span>Ctrl+K pour afficher ou masquer</span>
        </footer>
      </section>
    </div>,
    document.body,
  );
}
