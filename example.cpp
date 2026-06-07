typedef struct { 
char MSSV[10];
char Ten[30];
char Lop [10];
int NS;
int Khoa;
} #include <stdio.h>
#define N 2 //so SV
struct SinhVien{ 
char MSSV[10];
char Ten[30];
char Lop [10];
int NS;
int Khoa;
}; 
struct SinhVien SV[N];
int main()
{
    int i;
    for(i=0;i<N;++i)
        {
            printf("Nhap thong tin SV%d:\n",i);
            printf("MSSV của SV%d:",i); scanf("%s",SV[i].MSSV);
            printf("Ten của SV%d:",i); scanf("%s",SV[i].Ten);
            printf("Lop của SV%d:",i); scanf("%s",SV[i].Lop);
            printf("Nam sinh của SV%d:",i); scanf("%d",&SV[i].NS);
            printf("Khoa hoc của SV%d:",i); scanf("%d",&SV[i].Khoa);
            printf("------------------------\n");
        }
    for(i=0;i<N;++i)
        {
            printf("Thong tin cua SV%d:\n",i);
            printf("MSSV %s, Ten %s, SN %d, Lop %s, Khoa %d \n",SV[i].MSSV,SV[i].Ten,SV[i].NS,SV[i].Lop,SV[i].Khoa);
            printf("------------------------\n");
        }
    
    return 0;
}