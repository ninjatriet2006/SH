import { invokeCommand, type Empty, type Transaction } from './types';

export async function listUserTransactions(userId: string): Promise<Transaction[]> {
    try {
        const result = await invokeCommand<Transaction[], { user_id: string }>('list_user_transactions', {
            user_id: userId
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function listAllTransactions(): Promise<Transaction[]> {
    try {
        const result = await invokeCommand<Transaction[], Empty>('list_all_transactions', {});
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}
export async function deleteTransaction(id: string): Promise<void> {
    try {
        await invokeCommand<void, { id: string }>('delete_transaction', { id });
    } catch (error) {
        throw new Error(String(error));
    }
}
