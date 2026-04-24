module m;
  function int add(int a, int b);
  return a + b;
  endfunction
  int r;
  initial r = add (1, 2);
endmodule
// expected -----
module m;
  function int add(int a, int b);
  return a + b;
  endfunction
  int r;
  initial r = add(1, 2);
endmodule
