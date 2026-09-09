// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest';
import { upgradeSelectToCustomDropdown } from './customDropdown';

function setup(searchable = false) {
    document.body.innerHTML = `
        <label for="provider">Provider</label>
        <select id="provider">
            <option value="">Choose</option>
            <option value="a">Alpha</option>
            <option value="b">Beta</option>
        </select>`;
    const select = document.querySelector('select') as HTMLSelectElement;
    upgradeSelectToCustomDropdown(select, searchable);
    return {
        select,
        input: document.querySelector('[role="combobox"]') as HTMLInputElement,
        list: document.querySelector('[role="listbox"]') as HTMLElement,
    };
}

describe('custom dropdown', () => {
    it('selects by keyboard and dispatches bubbling events', () => {
        const { select, input } = setup();
        const onChange = vi.fn();
        document.body.addEventListener('change', onChange);

        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));

        expect(select.value).toBe('a');
        expect(input.value).toBe('Alpha');
        expect(onChange).toHaveBeenCalledOnce();
        expect(input.getAttribute('aria-expanded')).toBe('false');
    });

    it('filters searchable options and syncs disabled state', () => {
        const { select, input, list } = setup(true);
        input.click();
        input.value = 'bet';
        input.dispatchEvent(new Event('input', { bubbles: true }));

        const visible = Array.from(list.children).filter(child => (child as HTMLElement).style.display !== 'none');
        expect(visible).toHaveLength(1);
        expect(visible[0].textContent).toBe('Beta');

        select.disabled = true;
        (select as HTMLSelectElement & { _syncCustomDropdown(): void })._syncCustomDropdown();
        expect(input.disabled).toBe(true);
        expect(input.getAttribute('aria-disabled')).toBe('true');
    });
});
