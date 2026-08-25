import { create } from 'zustand';
import type { Transaction } from '../../../bridge/types';
import { listUserTransactions, listAllTransactions, deleteTransaction } from '../../../bridge/transaction_bridge';

interface TransactionState {
    transactions: Transaction[];
    isLoading: boolean;
    
    fetchUserTransactions: (userId: string) => Promise<void>;
    fetchAllTransactions: () => Promise<void>;
    removeTransaction: (id: string) => Promise<void>;
}

export const useTransactionStore = create<TransactionState>((set) => ({
    transactions: [],
    isLoading: false,

    fetchUserTransactions: async (userId) => {
        set({ isLoading: true });
        try {
            const data = await listUserTransactions(userId);
            set({ transactions: data });
        } catch (error) {
            console.error("Lỗi lấy lịch sử giao dịch:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    fetchAllTransactions: async () => {
        set({ isLoading: true });
        try {
            const data = await listAllTransactions();
            set({ transactions: data });
        } catch (error) {
            console.error("Lỗi lấy tất cả giao dịch:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    removeTransaction: async (id) => {
        try {
            await deleteTransaction(id);
            set((state) => ({
                transactions: state.transactions.filter(t => t.id !== id)
            }));
        } catch (error) {
            console.error("Lỗi khi xóa giao dịch:", error);
            throw error;
        }
    }
}));
