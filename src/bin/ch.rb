def canary arg
  p arg
end

def filter_path path_ary
  # p "from filter_path #{Hash[path_ary: path_ary].inspect}"
  path_ary.length >= 3
end
