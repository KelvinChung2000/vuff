module m;
  function automatic int   add( input  int a, input int b);
    return a   +  b;
  endfunction
  function int   compute(input int x);
    int   tmp;
    tmp = x + 1;
    return tmp * 2;
  endfunction
endmodule
// expected -----
module m;
  function automatic int add(input int a, input int b);
    return a + b;
  endfunction
  function int compute(input int x);
    int tmp;
    tmp = x + 1;
    return tmp * 2;
  endfunction
endmodule
