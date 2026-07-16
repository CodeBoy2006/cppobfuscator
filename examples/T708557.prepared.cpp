#include <iostream>
#include <utility>
#include <vector>
static std::vector<int> graph_storage[200005]; static std::vector<std::pair<int, int>> operation_storage;
static void traverse_tree(int node, int parent) { for (int next_node : graph_storage[node]) { if (next_node == parent) continue; operation_storage.emplace_back(node, next_node); operation_storage.emplace_back(next_node, node); operation_storage.emplace_back(node, node); traverse_tree(next_node, node); } }
static void run_case() { int node_count; std::cin >> node_count; for (int index = 1; index <= node_count; ++index) graph_storage[index].clear(); operation_storage.clear(); for (int index = 0; index < node_count - 1; ++index) { int from, to; std::cin >> from >> to; graph_storage[from].push_back(to); graph_storage[to].push_back(from); } operation_storage.emplace_back(1, 1); traverse_tree(1, 0); std::cout << operation_storage.size() << '\n'; for (auto &operation : operation_storage) std::cout << operation.first << ' ' << operation.second << '\n'; }
int main() { std::ios::sync_with_stdio(false); std::cin.tie(nullptr); std::cout.tie(nullptr); int test_count; std::cin >> test_count; while (test_count--) run_case(); return 0; }
