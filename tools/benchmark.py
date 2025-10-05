#!/usr/bin/env python3
"""
ProcessDistro Benchmark Tool

Benchmarks different task types and measures performance
across various node configurations.
"""

import asyncio
import json
import time
import statistics
from typing import List, Dict, Tuple


class BenchmarkSuite:
    def __init__(self):
        self.results = []

    def matrix_multiplication_benchmark(self, sizes: List[int]) -> Dict:
        """Benchmark matrix multiplication with different sizes"""
        results = {}

        for size in sizes:
            print(f"Benchmarking {size}x{size} matrix multiplication...")

            # Generate test matrices
            matrix_a = [[1.0 for _ in range(size)] for _ in range(size)]
            matrix_b = [[2.0 for _ in range(size)] for _ in range(size)]

            # Time the operation
            start_time = time.time()
            result = self.multiply_matrices(matrix_a, matrix_b)
            end_time = time.time()

            execution_time = end_time - start_time
            operations = size ** 3  # O(n^3) operations
            flops = operations / execution_time

            results[f"{size}x{size}"] = {
                "execution_time": execution_time,
                "operations": operations,
                "flops": flops
            }

            print(f"  Time: {execution_time:.3f}s, FLOPS: {flops:.2e}")

        return results

    def multiply_matrices(self, a: List[List[float]], b: List[List[float]]) -> List[List[float]]:
        """Simple matrix multiplication implementation"""
        n = len(a)
        result = [[0.0 for _ in range(n)] for _ in range(n)]

        for i in range(n):
            for j in range(n):
                for k in range(n):
                    result[i][j] += a[i][k] * b[k][j]

        return result

    def password_hashing_benchmark(self, password_counts: List[int]) -> Dict:
        """Benchmark password hashing with different batch sizes"""
        import hashlib

        results = {}
        test_password = "test_password_123"
        test_salt = "random_salt"

        for count in password_counts:
            print(f"Benchmarking {count} password hashes...")

            start_time = time.time()

            for i in range(count):
                # SHA256 hashing
                hasher = hashlib.sha256()
                hasher.update(f"{test_password}_{i}".encode())
                hasher.update(test_salt.encode())
                hash_result = hasher.hexdigest()

            end_time = time.time()

            execution_time = end_time - start_time
            hashes_per_second = count / execution_time

            results[f"{count}_passwords"] = {
                "execution_time": execution_time,
                "hashes_per_second": hashes_per_second,
                "count": count
            }

            print(
                f"  Time: {execution_time:.3f}s, Rate: {hashes_per_second:.1f} hashes/sec")

        return results

    def mandelbrot_benchmark(self, image_sizes: List[Tuple[int, int]]) -> Dict:
        """Benchmark Mandelbrot set generation"""
        results = {}

        for width, height in image_sizes:
            print(f"Benchmarking {width}x{height} Mandelbrot generation...")

            start_time = time.time()

            # Simple Mandelbrot calculation
            pixel_count = 0
            for y in range(height):
                for x in range(width):
                    # Map to complex plane
                    real = (x / width) * 3.0 - 2.0
                    imag = (y / height) * 2.0 - 1.0

                    # Calculate iterations
                    z_real, z_imag = 0.0, 0.0
                    iterations = 0
                    max_iter = 100

                    while iterations < max_iter and (z_real*z_real + z_imag*z_imag) < 4.0:
                        z_real_new = z_real*z_real - z_imag*z_imag + real
                        z_imag = 2*z_real*z_imag + imag
                        z_real = z_real_new
                        iterations += 1

                    pixel_count += 1

            end_time = time.time()

            execution_time = end_time - start_time
            pixels_per_second = pixel_count / execution_time

            results[f"{width}x{height}"] = {
                "execution_time": execution_time,
                "pixels_per_second": pixels_per_second,
                "total_pixels": pixel_count
            }

            print(
                f"  Time: {execution_time:.3f}s, Rate: {pixels_per_second:.1f} pixels/sec")

        return results

    def run_full_benchmark(self) -> Dict:
        """Run complete benchmark suite"""
        print("Starting ProcessDistro Benchmark Suite")
        print("=" * 50)

        # Matrix multiplication benchmarks
        print("\n1. Matrix Multiplication Benchmark")
        matrix_results = self.matrix_multiplication_benchmark(
            [64, 128, 256, 512])

        # Password hashing benchmarks
        print("\n2. Password Hashing Benchmark")
        password_results = self.password_hashing_benchmark(
            [100, 500, 1000, 5000])

        # Mandelbrot benchmarks
        print("\n3. Mandelbrot Generation Benchmark")
        mandelbrot_results = self.mandelbrot_benchmark([
            (256, 256),
            (512, 512),
            (1024, 1024)
        ])

        full_results = {
            "timestamp": time.time(),
            "matrix_multiplication": matrix_results,
            "password_hashing": password_results,
            "mandelbrot_generation": mandelbrot_results
        }

        return full_results

    def save_results(self, results: Dict, filename: str = "benchmark_results.json"):
        """Save benchmark results to file"""
        with open(filename, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\nResults saved to {filename}")


if __name__ == "__main__":
    benchmark = BenchmarkSuite()
    results = benchmark.run_full_benchmark()
    benchmark.save_results(results)
