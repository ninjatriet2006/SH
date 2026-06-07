#include <stdio.h>
#define N 8
void  xuatmang(int *pa, int len)
{
    for (int i = 0; i < len; i++)
    {
        printf("%d ", *(pa + i));
    }
    printf("\n");
}
void  nhapmang(int *pa, int len)
{
    for (int i = 0; i < len; i++)
    {
        printf("Nhap phan tu a[%d]: ", i);
        scanf("%d", pa + i);
    }
}
void    interchangesort(int *pa, int len)
{
    for (int i = 0; i < len - 1; i++)
    {
        for (int j = i + 1; j < len; j++)
        {
            if (*(pa + i) > *(pa + j))
            {
                int temp = *(pa + i);
                *(pa + i) = *(pa + j);
                *(pa + j) = temp;
            }
        }
    }
}
  
int linearsearch(int *pa, int len, int key)
{
    for (int i = 0; i < len; i++)
    {
        if (*(pa + i) == key)
        {
            return i;
        }
    }
    return -1;
}
int binarysearch(int *pa, int len, int key)
{
    int left = 0;
    int right = len - 1;
    while (left <= right)
    {
        int mid = left + (right - left) / 2;
        if (*(pa + mid) == key)
        {
            return mid;
        }
        else if (*(pa + mid) < key)
        {
            left = mid + 1;
        }
        else
        {
            right = mid - 1;
        }
    }
    return -1;
}
int main()
{
    int a[N];

    nhapmang(a, N);

    printf("Mang truoc khi sap xep:\n");
    xuatmang(a, N);

/*    int key;
    printf("Nhap khoa can tim kiem tuyen tinh: ");
    scanf("%d", &key);
    int pos_linear = linearsearch(a, N, key);
    if (pos_linear != -1)
    {
        printf("Tim thay %d tai vi tri index %d (tuyen tinh)\n", key, pos_linear);
    }
    else
    {
        printf("Khong tim thay %d (tuyen tinh)\n", key);
    }
*/
    interchangesort(a, N);
    printf("Mang sau khi sap xep:\n");
    xuatmang(a, N);
/*
    printf("Nhap khoa can tim kiem nhi phan: ");
    scanf("%d", &key);
    int pos_binary = binarysearch(a, N, key);
    if (pos_binary != -1)
    {
        printf("Tim thay %d tai vi tri index %d (nhi phan)\n", key, pos_binary);
    }
    else
    {
        printf("Khong tim thay %d (nhi phan)\n", key);
    }
*/
    return 0;
}