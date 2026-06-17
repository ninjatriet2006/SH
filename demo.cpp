/*######################################
# University of Information Technology #
# IT007 Operating System               #
# Huynh Hoang Gia, 24520413            #
# File: bai1.c                         #
######################################*/
#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include <semaphore.h>
#include <unistd.h>

sem_t sem_sells;
sem_t sem_products;

int sells = 0;
int products = 0;

void* processA(void* arg) {
    while (1) {
        sem_wait(&sem_sells);
        sells++;
        printf("Process A (Ban hang): sells = %d, products = %d\n", sells, products);
        sem_post(&sem_products);
        sleep(1);
    }
}

void* processB(void* arg) {
    while (1) {
        sem_wait(&sem_products);
        products++;
        printf("Process B (San xuat): sells = %d, products = %d\n", sells, products);
        sem_post(&sem_sells);
    }
}

int main() {
    pthread_t threadA, threadB;

    sem_init(&sem_sells, 0, 0);
    sem_init(&sem_products, 0, 23);

    pthread_create(&threadA, NULL, processA, NULL);
    pthread_create(&threadB, NULL, processB, NULL);

    pthread_join(threadA, NULL);
    pthread_join(threadB, NULL);

    sem_destroy(&sem_sells);
    sem_destroy(&sem_products);

    return 0;
}