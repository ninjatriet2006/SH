import { invoke } from '@tauri-apps/api/core';
import type { Transaction } from './types';

export async function listUserTransactions(userId: string): Promise<Transaction[]> {
    try {
        const result = await invoke<Transaction[]>('list_user_transactions', {
            userId: userId
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function listAllTransactions(): Promise<Transaction[]> {
    try {
        const result = await invoke<Transaction[]>('list_all_transactions');
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}
export async function deleteTransaction(id: string): Promise<void> {
    try {
        await invoke('delete_transaction', { id });
    } catch (error) {
        throw new Error(String(error));
    }
}
