import { Fragment } from 'react';
import {
  Menubar,
  MenubarMenu,
  MenubarTrigger,
  MenubarContent,
  MenubarItem,
  MenubarRadioGroup,
  MenubarRadioItem,
  MenubarSeparator,
  MenubarShortcut,
} from '@/components/ui/menubar';
import { Badge } from '@/components/ui/badge';
import { SF_MENUS, type MenuItem } from '@/menus/sf-menu-tree';
import { InfoTip } from './InfoTip';

export type MenuCommand =
  | 'file.new'
  | 'file.clear_token'
  | 'mode.spider'
  | 'config.open'
  | string;

interface Props {
  mode: 'spider' | 'list' | 'serp' | 'compare';
  onCommand: (id: MenuCommand) => void;
}

export function TopMenuBar({ mode, onCommand }: Props) {
  return (
    <Menubar>
      <span className="mr-2 pl-1 font-mono text-[11px] font-semibold uppercase tracking-widest text-primary">
        Rusting&nbsp;Frog
      </span>
      {SF_MENUS.map((m) => (
        <MenubarMenu key={m.key}>
          <MenubarTrigger>
            <span className="inline-flex items-center gap-1">
              {m.label}
              {m.docKey && <InfoTip field={m.docKey} className="opacity-60" />}
            </span>
          </MenubarTrigger>
          <MenubarContent>
            {m.key === 'mode' ? (
              <MenubarRadioGroup
                value={mode}
                onValueChange={(v) => {
                  if (v === 'spider') onCommand('mode.spider');
                }}
              >
                {m.items.map((it, i) =>
                  it.separator ? (
                    <MenubarSeparator key={i} />
                  ) : (
                    <MenubarRadioItem
                      key={it.label}
                      value={it.label.toLowerCase()}
                      disabled={it.disabled}
                    >
                      <span className="flex flex-1 items-center justify-between gap-3">
                        <span className="flex items-center gap-2">
                          {it.label}
                          {it.docKey && <InfoTip field={it.docKey} />}
                        </span>
                        {it.disabled && (
                          <Badge variant="outline" className="ml-auto">
                            Soon
                          </Badge>
                        )}
                      </span>
                    </MenubarRadioItem>
                  ),
                )}
              </MenubarRadioGroup>
            ) : (
              m.items.map((it, i) => (
                <Fragment key={`${m.key}-${i}`}>
                  {it.separator ? (
                    <MenubarSeparator />
                  ) : (
                    <MenuTreeItem
                      item={it}
                      onActivate={() => {
                        if (it.action?.kind === 'command') onCommand(it.action.id);
                      }}
                    />
                  )}
                </Fragment>
              ))
            )}
          </MenubarContent>
        </MenubarMenu>
      ))}
    </Menubar>
  );
}

function MenuTreeItem({ item, onActivate }: { item: MenuItem; onActivate: () => void }) {
  return (
    <MenubarItem disabled={item.disabled} onSelect={onActivate}>
      <span className="flex flex-1 items-center justify-between gap-3">
        <span className="flex items-center gap-2">
          {item.label}
          {item.docKey && <InfoTip field={item.docKey} />}
        </span>
        {item.shortcut && <MenubarShortcut>{item.shortcut}</MenubarShortcut>}
        {item.disabled && (
          <Badge variant="outline" className="ml-auto">
            Soon
          </Badge>
        )}
      </span>
    </MenubarItem>
  );
}
