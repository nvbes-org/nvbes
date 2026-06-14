import type { EnterpriseModuleGrant, EnterpriseRole } from '@nvbes/identity-client';
import { useEffect, useState } from 'react';
import { Button } from '../components/ui/button';
import { Checkbox } from '../components/ui/checkbox';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSet,
  FieldLegend,
} from '../components/ui/field';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from '../components/ui/sheet';
import { describeModuleGrant } from '../enterprise.permissions';

export type AccessFormValue = {
  role: EnterpriseRole;
  module_grants: EnterpriseModuleGrant[];
  workspace_ids: string[];
};

type UsersPageAccessProps = {
  open: boolean;
  title: string;
  roles: EnterpriseRole[];
  grants: EnterpriseModuleGrant[];
  workspaceIds: string[];
  value: AccessFormValue;
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (value: AccessFormValue) => void;
};

export function UsersPageAccess({
  open,
  title,
  roles,
  grants,
  workspaceIds,
  value,
  saving,
  onOpenChange,
  onSubmit,
}: UsersPageAccessProps) {
  const [draft, setDraft] = useState(value);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle>Edit access</SheetTitle>
          <SheetDescription>{title}</SheetDescription>
        </SheetHeader>

        <div className="flex flex-1 flex-col gap-5 px-4">
          <Field>
            <FieldLabel>Role</FieldLabel>
            <Select
              value={draft.role}
              onValueChange={(nextRole) =>
                setDraft((current) => ({ ...current, role: nextRole as EnterpriseRole }))
              }
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {roles.map((role) => (
                  <SelectItem key={role} value={role}>
                    {role}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          <CheckboxSet
            legend="Module grants"
            empty="No module grants are available."
            options={grants.map((grant) => ({ id: grant, label: describeModuleGrant(grant) }))}
            selected={draft.module_grants}
            onChange={(module_grants) => setDraft((current) => ({ ...current, module_grants }))}
          />

          <CheckboxSet
            legend="Workspace assignments"
            empty="No workspace assignments are present in the users payload yet."
            options={workspaceIds.map((id) => ({ id, label: id }))}
            selected={draft.workspace_ids}
            onChange={(workspace_ids) => setDraft((current) => ({ ...current, workspace_ids }))}
          />
        </div>

        <SheetFooter>
          <Button type="button" disabled={saving} onClick={() => onSubmit(draft)}>
            {saving ? 'Saving...' : 'Save access'}
          </Button>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}

function CheckboxSet<T extends string>({
  legend,
  empty,
  options,
  selected,
  onChange,
}: {
  legend: string;
  empty: string;
  options: Array<{ id: T; label: string }>;
  selected: T[];
  onChange: (selected: T[]) => void;
}) {
  return (
    <FieldSet>
      <FieldLegend>{legend}</FieldLegend>
      {options.length === 0 ? (
        <FieldDescription>{empty}</FieldDescription>
      ) : (
        <FieldGroup data-slot="checkbox-group" className="gap-3">
          {options.map((option) => (
            <Field key={option.id} orientation="horizontal">
              <Checkbox
                checked={selected.includes(option.id)}
                onCheckedChange={(checked) => {
                  onChange(
                    checked
                      ? [...selected, option.id]
                      : selected.filter((item) => item !== option.id),
                  );
                }}
              />
              <FieldContent>
                <FieldLabel>{option.label}</FieldLabel>
              </FieldContent>
            </Field>
          ))}
        </FieldGroup>
      )}
    </FieldSet>
  );
}
